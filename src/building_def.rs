use std::path::{Path, PathBuf};
use std::fs;
use std::fmt::{Display, Formatter, Write};
use std::io::Error as IOErr;

use crate::ini::{self, BuildingIni, MaterialMtl};
use crate::nmf::NmfInfo;


#[derive(Debug)]
pub struct BuildingDef {
    building_ini: PathBuf,
    renderconfig: PathBuf,

    model: PathBuf,
    model_lod: Option<PathBuf>,
    model_lod2: Option<PathBuf>,
    model_e: Option<PathBuf>,

    material: PathBuf,
    material_e: Option<PathBuf>,

    textures: Vec<PathBuf>,
}


#[derive(Debug)]
pub enum BuildingError {
    FileIO(PathBuf, IOErr),
    Parse(PathBuf, String),
    ModelMissing,
    MaterialMissing,
}


use crate::ini::BuildingToken as BT;
use crate::ini::RenderToken as RT;
use crate::ini::MaterialToken as MT;


impl BuildingDef {
    pub fn from_config(building_ini: &Path, renderconfig: &Path) -> Result<Self, BuildingError> {
        let render_root = renderconfig.parent().expect(&format!("Cannot get render root from {}", renderconfig.display()));

        let render_buf = fs::read_to_string(renderconfig).map_err(|e| BuildingError::FileIO(renderconfig.to_path_buf(), e))?;
        let render_ini = ini::parse_renderconfig_ini(&render_buf).map_err(|e| BuildingError::Parse(renderconfig.to_path_buf(), concat_parse_errors(e)))?;

        macro_rules! get_render_value {
            ($p:pat, $s:ident) => {{
                let mut res = None;
                for t in render_ini.tokens() {
                    match t {
                        $p => {
                            res = Some(render_root.join($s.as_str()));
                            break;
                        }, 
                        _ => ()
                    }
                }

                res
            }};
        }

        let model      = get_render_value!(RT::Model(s),            s).ok_or(BuildingError::ModelMissing)?;
        let model_lod  = get_render_value!(RT::ModelLod((s, _)),    s);
        let model_lod2 = get_render_value!(RT::ModelLod2((s, _)),   s);
        let model_e    = get_render_value!(RT::ModelEmissive(s),    s);

        let material   = get_render_value!(RT::Material(s),         s).ok_or(BuildingError::MaterialMissing)?;
        let material_e = get_render_value!(RT::MaterialEmissive(s), s);

        let mut textures = Vec::with_capacity(10);

        push_textures(&material, &mut textures)?;
        if let Some(ref material_e) = material_e {
            push_textures(material_e, &mut textures)?;
        }

        Ok(BuildingDef {
            building_ini: building_ini.to_path_buf(),
            renderconfig: renderconfig.to_path_buf(),
            model,
            model_lod,
            model_lod2,
            model_e,
            material,
            material_e,
            textures
        })
    }


    pub fn parse_and_validate(&self) -> Result<(), String> {
        let mut errors = String::with_capacity(0);

        macro_rules! check_path {
            ($name:expr, $path:expr) => { if !$path.exists() { writeln!(errors, "{} does not exist: {}", $name, $path.display()).unwrap(); }};
        }

        macro_rules! check_popt {
            ($name:expr, $path:expr) => { $path.as_ref().map(|p| check_path!($name, p)); };
        }

        check_path!("building.ini", &self.building_ini);
        check_path!("renderconfig.ini", &self.renderconfig);
        check_path!("MODEL", &self.model);
        check_popt!("MODEL_LOD", &self.model_lod);
        check_popt!("MODEL_LOD2", &self.model_lod2);
        check_popt!("MODELEMISSIVE", &self.model_e);
        check_path!("MATERIAL", &self.material);
        check_popt!("MATERIALEMISSIVE", &self.material_e);
        for tx in self.textures.iter() {
            check_path!("texture", tx);
        }

        let model = match NmfInfo::from_path(&self.model) {
            Ok(model) => Some(model),
            Err(e) => { 
                writeln!(errors, "Cannot load model nmf: {:?}", e).unwrap();
                None
            }
        };

        let mut str_buf = String::with_capacity(0);

        if let Some(model) = &model {
            macro_rules! push_errors {
                ($path:expr, $parser:expr, $pusher: ident, $pfx:expr) => {
                    let read_res = read_to_string_buf(&$path, &mut str_buf);
                    match read_res {
                        Ok(()) => match $parser(&str_buf) {
                            Ok(ini) => {
                                $pusher(&ini, model, &mut errors, $pfx)
                            },
                            Err(e) => writeln!(errors, "Cannot parse file {}: {:#?}", $path.display(), e).unwrap()
                        },
                        Err(e) => writeln!(errors, "Cannot read file {}: {:?}", $path.display(), e).unwrap()
                    };
                };
            }

            push_errors!(self.building_ini, ini::parse_building_ini, push_buildingini_errors, "building.ini");
            push_errors!(self.material, ini::parse_mtl, push_mtl_errors, "material");
            if let Some(material_e) = &self.material_e {
                push_errors!(material_e, ini::parse_mtl, push_mtl_errors, "emissive material");
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

}


fn push_buildingini_errors(building_ini: &BuildingIni, model: &NmfInfo, errors: &mut String, pfx: &'static str) {
    for t in building_ini.tokens() {
        macro_rules! check_model_node {
            ($node:ident, $cmp:ident) => {
                if model.objects.iter().all(|o| !o.name.as_str().$cmp($node.as_str())) {
                    writeln!(errors, "Error in {}: invalid token '{}', matching node was not found in the model nmf", pfx, t).unwrap();
                }
            };
        }

        match t {
            BT::StorageLivingAuto(id)          => check_model_node!(id, eq),
            BT::CostWorkBuildingNode(id)       => check_model_node!(id, eq),
            BT::CostWorkVehicleStationNode(id) => check_model_node!(id, eq),
            BT::CostWorkBuildingKeyword(key)   => check_model_node!(key, starts_with),
            _ => {}
        }

        // TODO: other checks
    }
}

fn push_mtl_errors(mtl: &MaterialMtl, model: &NmfInfo, errors: &mut String, pfx: &'static str) {
    let usage = model.get_submaterials_usage();

    // For now there is only 1 hard rule:
    // "all submaterials that are used by objects in NMF must have a token in mtl file"
    // other checks could be added later

    let used_by_objects = usage.iter().filter(|(_, i)| *i > 0);

    'obj_iter: for (obj_sm, _) in used_by_objects {
        for t in mtl.tokens() {
            match t {
                MT::Submaterial(mtl_sm) => {
                    if *obj_sm == mtl_sm.as_str() {
                        continue 'obj_iter;
                    }
                },
                _ => {}
            }
        }

        writeln!(errors, "Error in {}: NMF uses submaterial '{}', but the MTL file has no corresponding token", pfx, obj_sm).unwrap();
    }
}


fn concat_parse_errors(errors: Vec<(&str, String)>) -> String {
    let mut result = String::with_capacity(4 * 1024);
    for (chunk, err) in errors.iter() {
        writeln!(result, "Error: {}\nChunk: [{}]", err, chunk).unwrap();
    }

    result
}


fn push_textures(mtl_path: &Path, textures: &mut Vec<PathBuf>) -> Result<(), BuildingError> {
    use crate::cfg::APP_SETTINGS;

    let mtl_root = mtl_path.parent().expect(&format!("Cannot get mtl root from {}", mtl_path.display()));
    let mtl_buf = fs::read_to_string(mtl_path).map_err(|e| BuildingError::FileIO(mtl_path.to_path_buf(), e))?;
    let mtl = ini::parse_mtl(&mtl_buf).map_err(|e| BuildingError::Parse(mtl_path.to_path_buf(), concat_parse_errors(e)))?;
    for t in mtl.tokens() {
        let tx_path = match t {
            MT::Texture((_, s))         => Some(APP_SETTINGS.path_stock.join(s.as_str())),
            MT::TextureNoMip((_, s))    => Some(APP_SETTINGS.path_stock.join(s.as_str())),
            MT::TextureMtl((_, s))      => Some(mtl_root.join(s.as_str())),
            MT::TextureNoMipMtl((_, s)) => Some(mtl_root.join(s.as_str())),
            _ => None
        };

        if let Some(tx_path) = tx_path {
            if textures.iter().all(|x| *x != tx_path) {
                textures.push(tx_path);
            }
        }
    }

    Ok(())
}


pub fn read_to_string_buf(path: &Path, buf: &mut String) -> Result<(), IOErr> {
    use std::io::Read;
    use std::convert::TryInto;
    buf.truncate(0);

    let mut file = fs::File::open(path)?;
    let meta = file.metadata()?;
    let sz: usize = meta.len().try_into().expect("Cannot get file length");
    buf.reserve(sz);
    file.read_to_string(buf)?;
    Ok(())
}


impl Display for BuildingDef {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        macro_rules! w_optln {
            ($f:ident, $fstr:expr, $v:expr) => {
                if let Some(ref v) = $v {
                    write!($f, $fstr, v)
                } else {    
                    write!($f, $fstr, "<NONE>")
                }
            };
        }

        writeln!(f, "Building {{")?;
        writeln!(f, "    building.ini:     {:?}", self.building_ini)?;
        writeln!(f, "    renderconfig.ini: {:?}", self.renderconfig)?;
        writeln!(f, "    model:            {:?}", self.model)?;
        w_optln!(f, "    model_lod:        {:?}", self.model_lod)?;
        w_optln!(f, "    model_lod2:       {:?}", self.model_lod2)?;
        w_optln!(f, "    model_e:          {:?}", self.model_e)?;
        writeln!(f, "    material:         {:?}", self.material)?;
        w_optln!(f, "    material_e:       {:?}", self.material_e)?;

        writeln!(f, "    textures: [")?;
        for tx in self.textures.iter() {
            writeln!(f, "        {:?}", tx)?;
        }

        writeln!(f, "    ]\n}}")
    }
}


impl Display for BuildingError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        match self {
            BuildingError::FileIO(path, e) => write!(f, "File error ({}): {:?}", path.display(), e),
            BuildingError::Parse(path, e)  => write!(f, "Parse error ({}): {}", path.display(), e),
            BuildingError::ModelMissing    => write!(f, "Model is missing"),
            BuildingError::MaterialMissing => write!(f, "Model is missing"),
        }
    }
}