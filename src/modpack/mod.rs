use std::fs;
use std::path::{Path, PathBuf};

use const_format::concatcp;
use regex::Regex;
use normpath::{BasePath, BasePathBuf, PathExt};
use lazy_static::lazy_static;

use crate::building_def::{BuildingDef, BuildingError as DefError};
use crate::cfg::{APP_SETTINGS, RENDERCONFIG_INI, BUILDING_INI};
use crate::read_to_string_buf;


pub struct BuildingSource {
    def: BuildingDef,
    skins: Option<()>,
    actions: Option<()>,
}

#[derive(Debug)]
pub enum SourceError{
    NoRenderconfig,
    MultiRenderconfig,
    Def(DefError),
    Validation(String),
    RefRead(std::io::Error),
    RefParse,
}

const RENDERCONFIG_SOURCE: &str = "renderconfig.source";
const RENDERCONFIG_REF: &str = "renderconfig.ref";

pub fn read_validate_sources(source_dir: &Path) -> Result<Vec::<BuildingSource>, usize> {
    let mut result = Vec::<BuildingSource>::with_capacity(10000);

    let mut errors: usize = 0;

    let mut ref_buf = String::with_capacity(256);
    let mut rev_buf = Vec::<PathBuf>::with_capacity(100);
    let mut backlog = Vec::<PathBuf>::with_capacity(100);
    backlog.push(source_dir.to_path_buf());

    while let Some(mut path) = backlog.pop() {
        macro_rules! log_err {
            ($err:expr $(, $v:expr)*) => {{
                errors += 1;
                eprintln!("{}: {}", path.strip_prefix(source_dir).expect("Impossible: could not strip root prefix").display(), $err);
                $($v)*
            }};
        }

        path.push(BUILDING_INI);
        if path.exists() {
            // try to push this building source
            let bld_ini = path.clone();

            path.set_file_name(RENDERCONFIG_SOURCE);
            let render_src = if path.exists() { Some(path.clone()) } else { None }; 
            path.set_file_name(RENDERCONFIG_REF);
            let render_ref = if path.exists() { Some(path.clone()) } else { None };

            path.pop();

            let building_def = match (render_src, render_ref) {
                (None, None)       => Err(SourceError::NoRenderconfig), 
                (Some(_), Some(_)) => Err(SourceError::MultiRenderconfig),
                (Some(render_src), None) => BuildingDef::from_config(&bld_ini, &render_src, resolve_source_path).map_err(SourceError::Def),
                (None, Some(render_ref)) => get_def_from_ref(bld_ini, render_ref, &mut ref_buf),
            };

            let building_source = building_def.and_then(|def| {
                if let Err(e) = def.parse_and_validate() {
                    Err(SourceError::Validation(e))
                } else {
                    // TODO: skins and actions
                    let skins = None;
                    let actions = None;
                    // NOTE: debug
                    println!("{}\n{}", path.display(), def);
                    Ok(BuildingSource { def, skins, actions })
                }
            });

//Err("Could not find any renderconfig (neither source nor ref)"),
//Err(concatcp!("Building has both ", RENDERCONFIG_SOURCE ," and ", RENDERCONFIG_REF)),

            match building_source {
                Ok(bs) => result.push(bs),
                Err(e) => log_err!(format!("{:?}", e))
            }
        } else {
            // try to push sub-dirs to backlog
            path.pop();
            match fs::read_dir(&path) {
                Ok(r_d) => {
                    for dir_entry in r_d {
                        match dir_entry {
                            Ok(dir_entry) => match dir_entry.file_type() {
                                Ok(filetype) => if filetype.is_dir() {
                                    rev_buf.push(dir_entry.path());
                                },
                                Err(e) => log_err!(e)
                            },
                            Err(e) => log_err!(e)
                        }
                    }

                    while let Some(x) = rev_buf.pop() {
                        backlog.push(x);
                    }
                },
                Err(e) => log_err!(e)
            }
        }
    }

    if errors == 0 {
        Ok(result)
    } else {
        Err(errors)
    }
}


lazy_static! {
    static ref RX_REF: Regex = Regex::new(r"^(#(\d{10}/[^\s]+))|(~([^\s]+))|([^\r\n]+)").unwrap();
}

fn get_def_from_ref(bld_ini: PathBuf, mut render_ref: PathBuf, buf: &mut String) -> Result<BuildingDef, SourceError> {
    read_to_string_buf(&render_ref, buf).map_err(SourceError::RefRead)?;
    let caps = RX_REF.captures(buf).ok_or(SourceError::RefParse)?;
    if let Some(_c) = caps.get(4) {
        // stock, get def directly from stock buildings
        todo!("stock building def not implemented")
    } else {
        let mut root: BasePathBuf = if let Some(c) = caps.get(2) {
            // workshop
            Ok(APP_SETTINGS.path_workshop.join(c.as_str()))
        } else if let Some(c) = caps.get(5) {
            // relative path
            render_ref.pop();
            Ok(render_ref.normalize().unwrap().join(c.as_str()))
        } else {
            Err(SourceError::RefParse)
        }?;

        root.push(RENDERCONFIG_INI);
        BuildingDef::from_config(&bld_ini, root.as_path(), resolve_source_path).map_err(SourceError::Def)
    }
}



fn resolve_source_path(root: &BasePath, tail: &str) -> BasePathBuf {
    let mut iter = tail.chars();
    let pfx = iter.next().expect("resolve_source_path called with empty tail");
    match pfx {
        '#' => APP_SETTINGS.path_workshop.join(iter.as_str()),
        '~' => APP_SETTINGS.path_stock.join(iter.as_str()),
        _   => root.join(tail)
    }
}
