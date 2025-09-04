# Phase 1 Task Breakdown: Python Bindings Toolchain Setup

**Version:** 1.0
**Status:** Done
**Date:** 2025-09-03

---

## 1. Objective

The goal of this phase is to establish the complete end-to-end toolchain for building, testing, and packaging the Rust-based Anda DB functionality as a native Python module. By the end of this phase, we will have a minimal "hello world" function, callable from Python, built from our Rust codebase.

## 2. Task Breakdown

| Task ID | Task Description                                                                                                                                                                                          | Priority | Dependencies      | Estimated Effort | Status |
| :------ | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :------- | :---------------- | :--------------- | :----- |
| **P1-T1** | **Create New Rust Crate:** Create a new, library-focused Rust crate named `anda_py` within the `tools` directory. This crate will house all FFI and Python-specific logic. | High     | -                 | Low              | Done   |
| **P1-T2** | **Update Cargo Workspace:** Add the new `anda_py` crate to the `[workspace.members]` list in the root `Cargo.toml` file to ensure it is recognized by the workspace build system.                               | High     | P1-T1             | Very Low         | Done   |
| **P1-T3** | **Add `pyo3` Dependency:** In `anda_py/Cargo.toml`, add the `pyo3` crate as a dependency with the `extension-module` feature enabled to mark it as a Python extension.                                      | High     | P1-T1             | Very Low         | Done   |
| **P1-T4** | **Create `pyproject.toml`:** Create a `pyproject.toml` file at the project root. Configure it for `maturin` by specifying the Python module name (`anda`) and pointing to the `anda_py` crate.                 | High     | P1-T1             | Low              | Done   |
| **P1-T5** | **Implement "Hello World" Binding:** In `anda_py/src/lib.rs`, create a simple test function and expose it to Python using the `#[pyfunction]` and `#[pymodule]` macros from `pyo3`. | High     | P1-T3             | Low              | Done   |
| **P1-T6** | **Set Up Python Test Environment:** Create a `tests_py/` directory inside `tools/anda_py`. Inside, create a `test_bindings.py` file and a `requirements.txt` file.                 | Medium   | -                 | Low              | Done   |
| **P1-T7** | **Build & Test Integration:** Execute `maturin develop` to build the bindings and install them in a virtual environment. Run `pytest` to execute a test that imports `anda` and asserts the "hello world" function works correctly. | High     | P1-T4, P1-T5, P1-T6 | Low              | Done   |

---


## 3. Phase 2: Core Logic Implementation

**Objective:** To replace the "hello world" placeholder with the actual `execute_kip` function, connecting the Python interface to the core `anda_engine::memory` logic.

| Task ID | Task Description                                                                                                                                                                                          | Priority | Dependencies      | Estimated Effort | Status |
| :------ | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :------- | :---------------- | :--------------- | :----- |
| **P2-T1** | **Add `anda_engine` Dependency:** Add `anda_engine` as a dependency in `tools/anda_py/Cargo.toml` to allow calling the core KIP execution logic.                                                              | High     | P1-*              | Very Low         | To Do  |
| **P2-T2** | **Define `execute_kip` Function Signature:** In `tools/anda_py/src/lib.rs`, replace the `sum_as_string` function with the `execute_kip` function, matching the signature from the FRD (FR-02) using `pyo3` attributes. | High     | P1-*              | Medium           | To Do  |
| **P2-T3** | **Implement KIP Engine Call:** In the body of `execute_kip`, instantiate and invoke the KIP engine from `anda_engine::memory`. This includes passing the `command`, `parameters`, and `dry_run` arguments.          | High     | P2-T1, P2-T2      | High             | To Do  |
| **P2-T4** | **Handle Return Value Conversion:** Convert the successful result from the KIP engine (e.g., a Rust struct or `serde_json::Value`) into a Python dictionary (`PyDict`) and return it.                               | High     | P2-T3             | Medium           | To Do  |
| **P2-T5** | **Update Python Tests for Happy Path:** Modify `tests_py/test_bindings.py` to call `execute_kip` with a simple, valid command (e.g., `META version;`) and assert that it returns a dictionary as expected.        | High     | P2-T2             | Low              | To Do  |
| **P2-T6** | **Verify Phase 2 Completion:** Run `maturin develop` followed by `pytest` to ensure the entire happy-path workflow is functional, from the Python call to the Rust engine and back.                               | High     | P2-T4, P2-T5      | Low              | To Do  |