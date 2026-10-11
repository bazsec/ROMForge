use librashader_pack::ShaderPresetPack;
use librashader_presets::{ShaderFeatures, ShaderPreset};
use librashader_reflect::{back::CompileShader, reflect::ReflectShader};
use librashader_reflect::{
    back::targets::WGSL,
    front::SpirvCompilation,
    reflect::{
        naga::{Naga, NagaLoweringOptions},
        presets::CompilePresetTarget,
    },
};
fn main() -> anyhow::Result<()> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let preset = ShaderPreset::try_parse(
        root.join("assets/shaders/crt/crt-lottes.slangp"),
        ShaderFeatures::empty(),
    )?;
    let pack = ShaderPresetPack::load_from_preset::<anyhow::Error>(preset)?;
    let (passes, semantics) = WGSL::compile_preset_passes::<SpirvCompilation, Naga, anyhow::Error>(
        pack.passes,
        pack.textures.iter().map(|t| &t.meta),
    )?;
    for (_, mut compiler) in passes {
        let reflection = compiler.reflect(0, &semantics)?;
        println!("{reflection:#?}");
        let compiled = compiler.compile(NagaLoweringOptions {
            write_pcb_as_ubo: true,
            sampler_bind_group: 1,
            suppress_derivative_uniformity: true,
        })?;
        std::fs::write(
            root.join("crates/romforge-runtime/src/lottes.frag.wgsl"),
            format!(
                "// Generated from the pinned upstream CRT-Lottes by tools/compile-shader.\n// Timothy Lottes: public domain; see assets/shaders/crt/shaders/crt-lottes.slang.\n{}",
                compiled.fragment
            ),
        )?;
    }
    Ok(())
}
