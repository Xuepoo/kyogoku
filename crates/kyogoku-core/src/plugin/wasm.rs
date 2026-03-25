//! WebAssembly runtime for loading WASM plugins.
//!
//! This module provides safe execution of WASM plugins using wasmtime.

#[cfg(feature = "wasm-plugins")]
mod wasm_runtime {
    use anyhow::Result;
    use tracing::{debug, info};
    use wasmtime::*;

    use kyogoku_parser::TranslationBlock;

    use crate::plugin::{Plugin, PluginInfo, PluginType};

    /// A WASM plugin loaded into the runtime.
    pub struct WasmPlugin {
        info: PluginInfo,
        store: Store<()>,
        instance: Instance,
        memory: Memory,
    }

    impl WasmPlugin {
        /// Load a WASM plugin from disk.
        pub fn load(info: PluginInfo) -> Result<Self> {
            if info.plugin_type != PluginType::Wasm {
                anyhow::bail!("Plugin {} is not a WASM plugin", info.name);
            }

            let engine = Engine::default();
            let mut store = Store::new(&engine, ());

            // Load and compile the module
            let module = Module::from_file(&engine, &info.path).map_err(|e| {
                anyhow::anyhow!("Failed to load WASM module {}: {}", info.path.display(), e)
            })?;

            // Create linker with WASI-like imports (minimal for now)
            let linker = Linker::new(&engine);

            // Instantiate the module
            let instance = linker.instantiate(&mut store, &module).map_err(|e| {
                anyhow::anyhow!("Failed to instantiate WASM module {}: {}", info.name, e)
            })?;

            // Get memory export
            let memory = instance
                .get_memory(&mut store, "memory")
                .ok_or_else(|| anyhow::anyhow!("WASM module must export 'memory'"))?;

            info!("Loaded WASM plugin: {} v{}", info.name, info.version);

            Ok(Self {
                info,
                store,
                instance,
                memory,
            })
        }

        /// Call the parse function exported by the WASM module.
        ///
        /// Expected WASM exports:
        /// - `alloc(size: i32) -> i32`: Allocate memory
        /// - `dealloc(ptr: i32, size: i32)`: Free memory
        /// - `parse(ptr: i32, len: i32) -> i32`: Parse content, returns pointer to result
        /// - `get_result_len() -> i32`: Get length of last result
        fn call_parse(&mut self, content: &[u8]) -> Result<Vec<TranslationBlock>> {
            // Get exported functions
            let alloc = self
                .instance
                .get_typed_func::<i32, i32>(&mut self.store, "alloc")
                .map_err(|e| anyhow::anyhow!("Missing 'alloc' export: {}", e))?;

            let parse = self
                .instance
                .get_typed_func::<(i32, i32), i32>(&mut self.store, "parse")
                .map_err(|e| anyhow::anyhow!("Missing 'parse' export: {}", e))?;

            let get_result_len = self
                .instance
                .get_typed_func::<(), i32>(&mut self.store, "get_result_len")
                .map_err(|e| anyhow::anyhow!("Missing 'get_result_len' export: {}", e))?;

            // Allocate memory for input
            let input_len = content.len() as i32;
            let input_ptr = alloc.call(&mut self.store, input_len)?;

            // Copy input to WASM memory
            self.memory
                .write(&mut self.store, input_ptr as usize, content)?;

            // Call parse
            let result_ptr = parse.call(&mut self.store, (input_ptr, input_len))?;

            // Get result length
            let result_len = get_result_len.call(&mut self.store, ())? as usize;

            // Read result from WASM memory
            let mut result_buf = vec![0u8; result_len];
            self.memory
                .read(&self.store, result_ptr as usize, &mut result_buf)?;

            // Deserialize JSON result
            let blocks: Vec<TranslationBlock> = serde_json::from_slice(&result_buf)
                .map_err(|e| anyhow::anyhow!("Failed to deserialize WASM parse result: {}", e))?;

            debug!("WASM parse returned {} blocks", blocks.len());
            Ok(blocks)
        }

        /// Call the serialize function exported by the WASM module.
        fn call_serialize(
            &mut self,
            blocks: &[TranslationBlock],
            template: &[u8],
        ) -> Result<Vec<u8>> {
            // Get exported functions
            let alloc = self
                .instance
                .get_typed_func::<i32, i32>(&mut self.store, "alloc")
                .map_err(|e| anyhow::anyhow!("Missing 'alloc' export: {}", e))?;

            let serialize = self
                .instance
                .get_typed_func::<(i32, i32, i32, i32), i32>(&mut self.store, "serialize")
                .map_err(|e| anyhow::anyhow!("Missing 'serialize' export: {}", e))?;

            let get_result_len = self
                .instance
                .get_typed_func::<(), i32>(&mut self.store, "get_result_len")
                .map_err(|e| anyhow::anyhow!("Missing 'get_result_len' export: {}", e))?;

            // Serialize blocks to JSON
            let blocks_json = serde_json::to_vec(blocks)?;

            // Allocate memory for blocks JSON
            let blocks_ptr = alloc.call(&mut self.store, blocks_json.len() as i32)?;
            self.memory
                .write(&mut self.store, blocks_ptr as usize, &blocks_json)?;

            // Allocate memory for template
            let template_ptr = alloc.call(&mut self.store, template.len() as i32)?;
            self.memory
                .write(&mut self.store, template_ptr as usize, template)?;

            // Call serialize
            let result_ptr = serialize.call(
                &mut self.store,
                (
                    blocks_ptr,
                    blocks_json.len() as i32,
                    template_ptr,
                    template.len() as i32,
                ),
            )?;

            // Get result length
            let result_len = get_result_len.call(&mut self.store, ())? as usize;

            // Read result from WASM memory
            let mut result_buf = vec![0u8; result_len];
            self.memory
                .read(&self.store, result_ptr as usize, &mut result_buf)?;

            Ok(result_buf)
        }
    }

    impl Plugin for WasmPlugin {
        fn name(&self) -> &str {
            &self.info.name
        }

        fn version(&self) -> &str {
            &self.info.version
        }

        fn description(&self) -> &str {
            &self.info.description
        }

        fn extensions(&self) -> Vec<String> {
            self.info.extensions.clone()
        }

        fn parse(&self, _content: &[u8]) -> Result<Vec<TranslationBlock>> {
            // Note: This requires &mut self, but Plugin trait uses &self
            // We'll need to use interior mutability in practice
            // For now, this is a placeholder that documents the interface
            anyhow::bail!("WASM plugins require mutable access - use WasmPluginRunner instead")
        }

        fn serialize(&self, _blocks: &[TranslationBlock], _template: &[u8]) -> Result<Vec<u8>> {
            anyhow::bail!("WASM plugins require mutable access - use WasmPluginRunner instead")
        }
    }

    /// Runtime for executing WASM plugins with proper mutability.
    pub struct WasmPluginRunner {
        plugin: WasmPlugin,
    }

    impl WasmPluginRunner {
        pub fn new(info: PluginInfo) -> Result<Self> {
            let plugin = WasmPlugin::load(info)?;
            Ok(Self { plugin })
        }

        pub fn parse(&mut self, content: &[u8]) -> Result<Vec<TranslationBlock>> {
            self.plugin.call_parse(content)
        }

        pub fn serialize(
            &mut self,
            blocks: &[TranslationBlock],
            template: &[u8],
        ) -> Result<Vec<u8>> {
            self.plugin.call_serialize(blocks, template)
        }

        pub fn info(&self) -> &PluginInfo {
            &self.plugin.info
        }
    }
}

#[cfg(feature = "wasm-plugins")]
pub use wasm_runtime::WasmPluginRunner;

#[cfg(not(feature = "wasm-plugins"))]
mod wasm_stub {
    use anyhow::Result;
    use kyogoku_parser::TranslationBlock;

    use crate::plugin::PluginInfo;

    /// Stub for when WASM support is disabled.
    pub struct WasmPluginRunner;

    impl WasmPluginRunner {
        pub fn new(_info: PluginInfo) -> Result<Self> {
            anyhow::bail!(
                "WASM plugin support is not enabled. Rebuild with --features wasm-plugins"
            )
        }

        pub fn parse(&mut self, _content: &[u8]) -> Result<Vec<TranslationBlock>> {
            unreachable!()
        }

        pub fn serialize(
            &mut self,
            _blocks: &[TranslationBlock],
            _template: &[u8],
        ) -> Result<Vec<u8>> {
            unreachable!()
        }
    }
}

#[cfg(not(feature = "wasm-plugins"))]
pub use wasm_stub::WasmPluginRunner;
