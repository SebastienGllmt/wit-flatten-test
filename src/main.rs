use anyhow::Context;
use anyhow::bail;
use bytes::BytesMut;
use std::fs;
use std::path::Path;
use tokio_util::codec::Encoder;
use wasm_wave::{
    untyped::UntypedFuncCall,
    value::{FuncType, Type, Value, resolve_wit_func_type},
    wasm::WasmFunc,
};
use wit_encoder::{NestedPackage, packages_from_parsed};
use wit_parser::{PackageId, Resolve};
use wrpc_runtime_wasmtime::ValEncoder;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // create file

    // Parse all WIT files into a Resolve
    let (resolve, main_package) = get_resolve(Path::new("wit"))?;
    {
        // Create output directory if it doesn't exist
        fs::create_dir_all("out")?;
        // Call flatten_wit_files with all wit files and output path
        flatten_wit_files(&resolve, Path::new("out/flatten.wit"))?;
    }

    // try parsing file
    {
        let input = r#"queue-shader("")"#;
        let untyped_call = UntypedFuncCall::parse(input)?;
        let func_name = untyped_call.name().to_string();
        println!("func_name: {func_name}");
        let func_type = get_func_type(&resolve, &main_package, &func_name)?;
        println!("func_type: {func_type}");
        let param_types = func_type.params().collect::<Vec<_>>();
        let values = untyped_call.to_wasm_params::<Value>(&param_types)?;
        println!("values: {values:?}");

        let mut buf = BytesMut::default();
        // you can encode as long as you have a store to get the context from
        // let mut enc = ValEncoder::new(store.as_context_mut(), &param_types[0].into(), &vec![]);
        // enc.encode(&values[0].into(), &mut buf)
        //     .with_context(|| format!("failed to encode result value"))?;
    }

    Ok(())
}

fn get_func_type(
    resolve: &Resolve,
    pkg_id: &PackageId,
    func_name: &str,
) -> anyhow::Result<FuncType> {
    let world_id = resolve.select_world(&[*pkg_id], None)?;

    let key = wit_parser::WorldKey::Name(func_name.to_string());
    let world_item = resolve.worlds[world_id]
        .exports
        .get(&key)
        .ok_or_else(|| anyhow::anyhow!("function '{func_name}' not found in world exports"))?;
    let func = match world_item {
        wit_parser::WorldItem::Function(func) => func,
        _ => return Err(anyhow::anyhow!("'{func_name}' is not a function")),
    };
    resolve_wit_func_type(&resolve, func)
        .map_err(|e| anyhow::anyhow!("failed to resolve function type for '{func_name}': {e}"))
}

fn get_resolve(path: &Path) -> anyhow::Result<(Resolve, PackageId)> {
    let mut resolve = Resolve::new();
    let (main, _) = resolve.push_dir(path)?;
    Ok((resolve, main))
}

fn flatten_wit_files(resolve: &Resolve, output_path: &Path) -> anyhow::Result<()> {
    // Convert Resolve to wit-encoder Package structures
    let packages = packages_from_parsed(&resolve);

    if packages.is_empty() {
        bail!("no packages found");
    }

    let mut output = String::new();

    // All other packages
    for package in packages.iter() {
        let mut nested = NestedPackage::new(package.name().clone());
        // Copy all items from the original package to nested
        for item in package.items() {
            nested.item(item.clone());
        }
        output.push_str(&nested.to_string());
        output.push_str("\n");
    }

    // Save to disk
    fs::write(output_path, output)?;

    Ok(())
}
