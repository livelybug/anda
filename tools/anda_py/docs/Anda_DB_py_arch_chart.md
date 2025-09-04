# Anda DB Python Bindings Architecture

This document contains Mermaid.js diagrams visualizing the architecture and operational flow of the Anda DB Python bindings.

## 1. High-Level Workflow

This flowchart shows the end-to-end process of a function call from Python to the Rust backend and back.

```mermaid
flowchart TD
 subgraph PythonInput["Python Layer - Input"]
        B["Call anda.execute_kip"]
        A["Python User Script"]
  end
 subgraph PyO3Bridge["PyO3 Bridge"]
        C["Rust execute_kip function"]
  end
 subgraph RustBackend["Rust Backend"]
        D["RUNTIME.block_on"]
        E["Create Request and BaseCtx"]
        F["Call MEMORY.call"]
        G["Result"]
  end
 subgraph ResultHandling["Result Handling"]
        H["Acquire Python GIL"]
        I["pyo3 serde to_python conversion"]
        J["Return PyObject dict"]
        K["Create PyErr"]
        L["Raise Python Exception"]
  end
 subgraph PythonOutput["Python Layer - Output"]
        M["Receive Python Dictionary"]
        N["Catch ValueError"]
  end
    A --> B
    B --> C
    C --> D
    D --> E
    E --> F
    F --> G
    G -- Ok ToolOutput --> H
    H --> I
    I --> J
    G -- Err --> K
    K --> L
    J --> M
    L --> N
```

## 2. Detailed Sequence Diagram

This diagram illustrates the specific interactions and lifecycle of objects during a single `execute_kip` call.

```mermaid
sequenceDiagram
    actor PythonUser
    participant AndaModule as anda (Python)
    participant RustFFI as execute_kip (Rust)
    participant TokioRuntime as Tokio Runtime
    participant MemoryMgmt as MemoryManagement

    PythonUser->>AndaModule: execute_kip("META version")
    
    activate AndaModule
    AndaModule->>RustFFI: Call with Python args
    deactivate AndaModule
    
    activate RustFFI
    RustFFI->>TokioRuntime: block_on(async block)
    
    activate TokioRuntime
    TokioRuntime->>MemoryMgmt: call(ctx, request)
    
    activate MemoryMgmt
    Note over MemoryMgmt: Performs core KIP logic,<br/>interacts with CognitiveNexus & AndaDB.
    MemoryMgmt-->>TokioRuntime: Ok(ToolOutput)
    deactivate MemoryMgmt
    
    TokioRuntime-->>RustFFI: Returns Result<ToolOutput>
    deactivate TokioRuntime
    
    RustFFI->>RustFFI: pyo3 serde to_python(output)
    
    alt Happy Path
        RustFFI-->>PythonUser: Returns Python dict
    else Error Path
        Note over MemoryMgmt: On failure, returns Err(e)
        MemoryMgmt-->>TokioRuntime: Err(e)
        TokioRuntime-->>RustFFI: Returns Result Err(e)
        RustFFI-->>PythonUser: Raises ValueError exception
    end
    
    deactivate RustFFI
```
