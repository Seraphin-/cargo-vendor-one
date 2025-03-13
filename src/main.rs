pub fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().filter(|name| name != "vendor-one").skip(1).collect();
    if args.len() == 0 || args.clone().into_iter().find(|name| ((name == "--help") || (name == "-h"))).is_some() {
        println!("Usage: cargo vendor-one package1[@version1] [package2[@version2] ...]");
        return Ok(());
    }

    let vendored = cargo_vendor_one::vendor(args)?;

    println!("Vendored {} packages", vendored.len());
    for info in vendored {
        println!("{} => {}", info.request, info.path);
    }

    Ok(())
}
