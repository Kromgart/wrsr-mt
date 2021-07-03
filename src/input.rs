//use std::env;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::io::Read;
use std::convert::TryInto;

use regex::Regex;
use lazy_static::lazy_static;
use const_format::concatcp;

use crate::{
    StockBuilding, RenderConfig, StockBuildingsMap,
    BuildingDef, MaterialDef, 
    Skin, SkinMaterial, PathPrefix, IniTokenTexture,

    grep_ini_token, get_texture_tokens, get_texture_tokens_ext, 
    resolve_prefixed_path, read_to_string_buf,

    SRX_PATH_PREFIX, SRX_PATH, SRX_EOL, 
    PATH_ROOT_MODS,
    MAX_BUILDINGS,
    };


#[derive(Debug)]
enum SourceType<'a> {
    Stock(&'a str),
    Mod(PathBuf),
}


pub(crate) fn read_validate_sources<'stock>(src: &Path, stock_buildings: &mut StockBuildingsMap<'stock>) -> Vec<BuildingDef<'stock>> {

    let mut buf_sources = String::with_capacity(512);
    let mut pathbuf = src.to_path_buf();
    let mut data: Vec<BuildingDef<'stock>> = Vec::with_capacity(1000);

    push_buildings(&mut pathbuf, &mut data, &mut buf_sources, stock_buildings, &mut String::with_capacity(10));

    assert!(data.len() <= MAX_BUILDINGS);

    data
}


fn push_buildings<'stock>(pathbuf: &mut PathBuf, 
                          data: &mut Vec<BuildingDef<'stock>>,
                          buf_sources: &mut String,
                          stock_buildings: &mut StockBuildingsMap<'stock>,
                          indent: &mut String
                          )
{
    lazy_static! {
        static ref RX_SOURCE_STOCK: Regex = Regex::new(r"^#([_[:alnum:]]+)").unwrap();
        static ref RX_SOURCE_MOD: Regex = Regex::new(r"^[0-9]{10}\\[_[:alnum:]]+").unwrap();
    }

    // NOTE: Debug
    // println!("+++ {:?} +++", &pathbuf);

    pathbuf.push("building.source");

    if pathbuf.exists() {
        // leaf dir (building)

        buf_sources.clear();
        File::open(&pathbuf).unwrap().read_to_string(buf_sources).unwrap();
        pathbuf.pop(); //pop .source

        println!("{}* {}", indent, pathbuf.file_name().unwrap().to_str().unwrap());

        let src_type: SourceType = {
            if let Some(src_stock) = RX_SOURCE_STOCK.captures(&buf_sources) {
                SourceType::Stock(src_stock.get(1).unwrap().as_str())
            } else if let Some(src_mod) = RX_SOURCE_MOD.find(&buf_sources) {
                SourceType::Mod(PATH_ROOT_MODS.join(src_mod.as_str()))
            } else {
                panic!("Cannot parse building source ({:?})", &buf_sources);
            }
        };

        data.push(source_to_def(pathbuf, src_type, stock_buildings));

        return;
    } else {
        pathbuf.pop();

        println!("{}{}", indent, pathbuf.file_name().unwrap().to_str().unwrap());

        for subdir in get_subdirs(&pathbuf) {
            let dir_name = subdir.file_name();
            if dir_name.to_str().unwrap().starts_with("_") {
                continue;
            }

            let old_indent = indent.len();
            indent.push_str("  ");
            pathbuf.push(dir_name);

            push_buildings(pathbuf, data, buf_sources, stock_buildings, indent);

            pathbuf.pop();
            indent.truncate(old_indent);
        }
    }
}

fn get_subdirs(path: &PathBuf) -> impl Iterator<Item=fs::DirEntry>
{
    fs::read_dir(path)
        .unwrap()
        .map(|x| x.unwrap())
        .filter(|x| x.file_type().unwrap().is_dir())
}


fn source_to_def<'ini, 'map>(pathbuf: &mut PathBuf, source_type: SourceType, hmap: &'map mut StockBuildingsMap<'ini>) -> BuildingDef<'ini> {
    let mut def = match source_type {
        SourceType::Stock(key) => {
            get_stock_building(&key, hmap).unwrap()
        },
        SourceType::Mod(mut bld_dir_path) => {
            bld_dir_path.push("renderconfig.ini");
            parse_ini_to_def(RenderConfig::Mod(bld_dir_path))
        }
    };
    

    // TODO: overriding with custom files (if they exist in dir):
    // ---------------------------
    
    pathbuf.push("building.ini");
    if pathbuf.exists() { 
        def.building_ini.push(&pathbuf) 
    }

    pathbuf.set_file_name("imagegui.png");
    if pathbuf.exists() {
        def.imagegui.replace(pathbuf.clone());
    }

    pathbuf.set_file_name("building.skins");
    if pathbuf.exists() {
        def.skins = get_skins(&pathbuf);
    }

    pathbuf.set_file_name("material.mtlx");
    if pathbuf.exists() {
        def.material.render_token.value.push(&pathbuf);
        def.material.textures = get_texture_tokens_ext(&pathbuf);
    }

    pathbuf.set_file_name("material_e.mtlx");
    if pathbuf.exists() {
        if let Some(ref mut mat_e) = def.material_emissive {
            mat_e.render_token.value.push(&pathbuf);
            mat_e.textures = get_texture_tokens_ext(pathbuf);
        }
        else {
            panic!("Trying to override material_e, while renderconfig does not have it");
        }
    }

    pathbuf.pop();
    // -----------------------------

    // NOTE: Debug
    //println!("{}", &def);

    def.validate_paths();
    def
}


fn get_stock_building<'a, 'ini, 'map>(key: &'a str, hmap: &'map mut StockBuildingsMap<'ini>) -> Option<BuildingDef<'ini>> {
    if let Some(mref) = hmap.get_mut(key) {
        match mref {
            (_, StockBuilding::Parsed(ref x)) => Some(x.clone()),
            (k, StockBuilding::Unparsed(y)) => {
                let x = parse_ini_to_def(RenderConfig::Stock { key: k, data: y }); 
                *mref = (k, StockBuilding::Parsed(x.clone()));
                Some(x)
            }
        }
    } else { None }
}

fn parse_ini_to_def<'ini>(render_config: RenderConfig<'ini>) -> BuildingDef<'ini> {

    lazy_static! {
        static ref RX_MODEL:      Regex = Regex::new(concatcp!(r"(?m)^\sMODEL\s+?",            SRX_PATH, SRX_EOL)).unwrap();
        static ref RX_MODEL_LOD1: Regex = Regex::new(concatcp!(r"(?m)^\sMODEL_LOD\s+?",        SRX_PATH, SRX_EOL)).unwrap();
        static ref RX_MODEL_LOD2: Regex = Regex::new(concatcp!(r"(?m)^\sMODEL_LOD2\s+?",       SRX_PATH, SRX_EOL)).unwrap();
        static ref RX_MODEL_E:    Regex = Regex::new(concatcp!(r"(?m)^\sMODELEMISSIVE\s+?",    SRX_PATH, SRX_EOL)).unwrap();

        static ref RX_MATERIAL:   Regex = Regex::new(concatcp!(r"(?m)^\sMATERIAL\s+?",         SRX_PATH, SRX_EOL)).unwrap();
        static ref RX_MATERIAL_E: Regex = Regex::new(concatcp!(r"(?m)^\sMATERIALEMISSIVE\s+?", SRX_PATH, SRX_EOL)).unwrap();
    }

    let mut buf_mod_renderconfig = String::with_capacity(0);
    let root_path = render_config.root_path();

    let (render_source, building_ini, bbox, fire) = match render_config {
        RenderConfig::Stock { key, data } => {
            let mut building_ini = root_path.join("buildings_types");

            let bbox = building_ini.join(format!("{}.bbox", key));
            let fire = building_ini.join(format!("{}.fire", key));
            building_ini.push(format!("{}.ini", key));

            (data, building_ini, bbox, fire)
        },
        RenderConfig::Mod(ref cfg_path) => {
            read_to_string_buf(cfg_path.as_path(), &mut buf_mod_renderconfig);

            let bld_ini = root_path.join("building.ini");
            let bbox    = root_path.join("building.bbox");
            let fire    = root_path.join("building.fire");

            (buf_mod_renderconfig.as_str(), bld_ini, bbox, fire)
        }
    };

    let fire = if fire.exists() { Some(fire) } else { None };

    let model =          grep_ini_token(&RX_MODEL,      render_source, root_path).unwrap();
    let model_lod1 =     grep_ini_token(&RX_MODEL_LOD1, render_source, root_path);
    let model_lod2 =     grep_ini_token(&RX_MODEL_LOD2, render_source, root_path);
    let model_emissive = grep_ini_token(&RX_MODEL_E,    render_source, root_path);

    let material = MaterialDef::new(grep_ini_token(&RX_MATERIAL, render_source, root_path).unwrap());
    let material_emissive = grep_ini_token(&RX_MATERIAL_E, render_source, root_path).map(|x| MaterialDef::new(x));

    BuildingDef { 
        render_config, building_ini, bbox, fire, imagegui: None,
        model, model_lod1, model_lod2, model_emissive, 
        material, material_emissive, skins: Vec::with_capacity(0)
    }
}



fn get_skins(skinfile_path: &PathBuf) -> Vec<Skin> {
    lazy_static! {
        static ref RX: Regex = Regex::new(concatcp!(r"(?m)^", SRX_PATH_PREFIX, SRX_PATH, r"(\s+?\+\s+?", SRX_PATH_PREFIX, SRX_PATH, r")?\r\n")).unwrap();
    }

    // TODO: can estimate better (check file size)
    let mut result = Vec::with_capacity(8);
    let cfg = fs::read_to_string(skinfile_path).unwrap();
    let skinfile_dir = skinfile_path.parent().unwrap();

    for cap in RX.captures_iter(&cfg) {
        let type1: PathPrefix = cap.get(1).unwrap().as_str().try_into().unwrap();
        let path1 = cap.get(2).unwrap().as_str();
        let m_path = resolve_prefixed_path(type1, path1, skinfile_dir);

        let material = SkinMaterial { 
            path: m_path.to_path_buf(),
            textures: get_material_textures(&m_path) 
        };

        let material_emissive = cap.get(4).map(|x| {
            let type2: PathPrefix = x.as_str().try_into().unwrap();
            let path2 = cap.get(5).unwrap().as_str();
            let m_path = resolve_prefixed_path(type2, path2, skinfile_dir);

            SkinMaterial {
                path: m_path.to_path_buf(),
                textures: get_material_textures(&m_path)
            }
        });

        result.push(Skin { material, material_emissive });
    }

    result
}


fn get_material_textures(material_path: &Path) -> Vec<IniTokenTexture> {
    let ext = material_path.extension().unwrap();

    match ext.to_str().unwrap() {
        "mtl" => get_texture_tokens(material_path),
        "mtlx" => get_texture_tokens_ext(material_path),
        e => panic!("Unknown material extension '{}'", e)
    }
}
