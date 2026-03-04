# OCaml -> C ABI DLL on Windows: validated path

This engine now uses a direct OCaml build path that is proven to work on this machine:

1. `dune build`
2. `ocamlfind ocamlopt ... -output-complete-obj -o dist/dvc_coreengine_ocaml01.dll ... src/dvc_engine.c`
3. Build C conformance tests against that DLL.
4. Run `api_smoke`, `api_closure`, and `api_conformance_ct`.

## Critical toolchain note

On this environment, long link command wrappers can fail unless:

- `TEMP=tmp/`
- `TMP=tmp/`

are set before the `ocamlopt` link step.

## Local proof artifacts

- Minimal standalone repro: `tmp/ocaml_c_abi_repro/`
  - `hello.ml`
  - `hello_shim.c`
  - `host.c`
  - `build_and_run.cmd`
- Engine release build: `build_release.cmd`

## Primary references

- OCaml manual: Interfacing C with OCaml (runtime startup + embedding + callbacks):  
  https://ocaml.org/manual/5.2/intfc.html
- OCaml manual (`-output-complete-obj` / `-output-obj` behavior):  
  https://ocaml.org/manual/5.2/intfc.html
- FlexDLL README (`FLEXDLL_RELOCATE` callback contract, Windows loader context):  
  https://github.com/ocaml/flexdll/blob/master/README.md
- Dune foreign stubs reference (C integration in OCaml libs):  
  https://dune.readthedocs.io/en/stable/reference/foreign-stubs.html

