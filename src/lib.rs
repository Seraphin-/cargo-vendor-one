use anyhow::{anyhow, Result};
use cargo::{
    core::{
        package::{Package, PackageSet},
        registry::PackageRegistry,
        resolver::{features::CliFeatures, HasDevUnits},
        shell::Verbosity,
        PackageId, Resolve, Workspace,
    },
    ops::{get_resolved_packages, load_pkg_lockfile, resolve_with_previous},
    util::important_paths::find_root_manifest_for_wd,
    GlobalContext,
};

use cargo::sources::SourceConfigMap;
use cargo::util::cache_lock::CacheLockMode::DownloadExclusive;
use fs_extra::dir::{copy, CopyOptions};
use semver::VersionReq;
use std::{
    fs,
    path::{Path, PathBuf},
};

use toml_edit::{value, DocumentMut};

fn setup_gctx() -> Result<GlobalContext> {
    let gctx = GlobalContext::default()?;
    gctx.shell().set_verbosity(Verbosity::Quiet);
    Ok(gctx)
}

fn find_cargo_toml(path: &Path) -> Result<PathBuf> {
    let path = fs::canonicalize(path)?;
    find_root_manifest_for_wd(&path)
}

fn fetch_workspace<'gctx>(
    gctx: &'gctx GlobalContext,
    path: &Path,
) -> Result<Workspace<'gctx>> {
    Workspace::new(path, gctx)
}

fn resolve_ws<'a>(ws: &Workspace<'a>) -> Result<(PackageSet<'a>, Resolve)> {
    let scm = SourceConfigMap::new(ws.gctx())?;
    let mut registry = PackageRegistry::new_with_source_config(ws.gctx(), scm)?;

    registry.lock_patches();
    let resolve = {
        let prev = load_pkg_lockfile(ws)?;
        let resolve: Resolve = resolve_with_previous(
            &mut registry,
            ws,
            &CliFeatures::new_all(true),
            HasDevUnits::No,
            prev.as_ref(),
            None,
            &[],
            false,
        )?;
        resolve
    };
    let packages = get_resolved_packages(&resolve, registry)?;
    Ok((packages, resolve))
}

fn get_id(
    name: &str,
    version: &Option<VersionReq>,
    resolve: &Resolve,
) -> Result<PackageId> {
    let mut matched_dep = Err(anyhow!("Unable to find package {name}"));
    for dep in resolve.iter() {
        if dep.name().as_str() == name
            && version
                .as_ref()
                .map_or(true, |ver| ver.matches(dep.version()))
        {
            if matched_dep.is_err() {
                matched_dep = Ok(dep);
            } else {
                eprintln!("There are multiple versions of {name} available. Try specifying a version.");
            }
        }
    }
    matched_dep
}

fn copy_package(pkg: &Package) -> Result<PathBuf> {
    fs::create_dir_all("vendor/")?;
    let options = CopyOptions::new();
    if let Some(name) = pkg.root().file_name() {
        let buf = PathBuf::from("vendor/");
        let buf = buf.join(name);
        if fs::exists(&buf)? {
            let _ = fs::remove_dir_all(&buf);
        }
        let _ = copy(pkg.root(), "vendor/", &options)?;
        Ok(buf.canonicalize()?)
    } else {
        Err(anyhow!("Dependency Folder does not have a name"))
    }
}

pub struct VendoredInfo {
    pub request: String,
    pub path: String
}

pub fn vendor<I: IntoIterator<Item=String>>(pkgs: I) -> Result<Vec<VendoredInfo>> {
    let gctx = setup_gctx()?;
    let _lock = gctx.acquire_package_cache_lock(DownloadExclusive)?;
    let workspace_path = find_cargo_toml(&PathBuf::from("."))?;
    let workspace = fetch_workspace(&gctx, &workspace_path)?;
    let (pkg_set, resolve) = resolve_ws(&workspace)?;

    let manifest_string = fs::read_to_string(&workspace_path)?;
    let mut manifest = manifest_string.parse::<DocumentMut>().expect("Invalid Cargo.toml?");

    let mut info = vec![];

    for pkg_req in pkgs {
        let req_info: Vec<&str> = pkg_req.split('@').collect();
        let mut version = None;
        if req_info.len() == 2 {
            version = Some(VersionReq::parse(req_info.get(1).unwrap())?);
        }
        let pkg = req_info.get(0).unwrap();

        let package = pkg_set.get_one(get_id(pkg, &version, &resolve)?)?;
        let path = copy_package(package)?;
        let path_str = path.to_str().unwrap();
        let source = package.package_id().source_id().display_registry_name();

        manifest["patch"][&source][pkg]["path"] = value(path_str);
        if version.is_some() {
            manifest["patch"][source][pkg]["version"] = value(version.unwrap().to_string());
        }

        info.push(VendoredInfo { request: pkg_req, path: path_str.to_string() });
    }

    fs::write(workspace_path, manifest.to_string())?;

    Ok(info)
}
