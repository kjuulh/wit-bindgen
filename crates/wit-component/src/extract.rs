use crate::{decode_interface_component, ComponentInterfaces};
use anyhow::{bail, Context, Result};

/// Result of extracting interfaces embedded within a core wasm file.
///
/// This structure is reated by the [`extract_module_interfaces`] function.
#[derive(Default)]
pub struct ModuleInterfaces<'a> {
    /// The core wasm binary with custom sections removed.
    pub wasm: Vec<u8>,

    /// The interfaces found within the original component.
    pub interfaces: ComponentInterfaces<'a>,
}

/// This function will parse the `wasm` binary given as input and return a
/// [`ModuleInterfaces`] which extracts the custom sections describing
/// component-level types from within the binary itself.
///
/// This is used to parse the output of `wit-bindgen`-generated modules and is
/// one of the earliest phases in transitioning such a module to a component.
/// The extraction here provides the metadata necessary to continue the process
/// later on.
pub fn extract_module_interfaces(wasm: &[u8]) -> Result<ModuleInterfaces<'_>> {
    let mut ret = ModuleInterfaces::default();

    for payload in wasmparser::Parser::new(0).parse_all(wasm) {
        match payload.context("decoding item in module")? {
            wasmparser::Payload::CustomSection(cs) => {
                if !cs.name().starts_with("component-type") {
                    continue;
                }
                ret.decode(cs.data())
                    .with_context(|| format!("decoding custom section {}", cs.name()))?;
            }
            _ => {}
        }
    }

    // TODO: should remove the custom setions decoded above from the wasm binary
    // created here, and bytecodealliance/wasmparser#792 should help with that
    // to make the loop above pretty small.
    ret.wasm = wasm.to_vec();

    Ok(ret)
}

impl<'a> ModuleInterfaces<'a> {
    fn decode(&mut self, component: &'a [u8]) -> Result<()> {
        let ComponentInterfaces {
            default,
            imports,
            exports,
        } = decode_interface_component(component)?;

        if let Some(iface) = default {
            if self.interfaces.default.is_some() {
                bail!("default interface specified a second time");
            }
            self.interfaces.default = Some(iface);
        }

        // TODO: instead of returning an error here on duplicate interfaces
        // this should merge the two interfaces. This probably requires world
        // files to exist first though.

        for (name, import) in imports {
            if self.interfaces.imports.contains_key(&name) {
                bail!("import interface `{name}` specified twice");
            }
            self.interfaces.imports.insert(name, import);
        }
        for (name, export) in exports {
            if self.interfaces.exports.contains_key(&name) {
                bail!("export interface `{name}` specified twice");
            }
            self.interfaces.exports.insert(name, export);
        }
        Ok(())
    }
}