//use std::env;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::ops::Range;
use std::convert::{TryFrom, TryInto};

use regex::Regex;
use lazy_static::lazy_static;
use const_format::concatcp;

use crate::cfg::{AppSettings, APP_SETTINGS};
use crate::nmf;

//--------------------------------------------
//                SOURCES

#[derive(Debug, Clone)]
pub struct BuildingDef<'stock> {
    pub render_config: RenderConfig<'stock>,

    pub building_ini: PathBuf,
    pub bbox: PathBuf,
    pub fire: Option<PathBuf>,
    pub imagegui: Option<PathBuf>,

    pub model: ModelDef,
    pub model_lod1: Option<ModelDef>,
    pub model_lod2: Option<ModelDef>,
    pub model_emissive: Option<ModelDef>,

    pub material: MaterialDef,
    pub material_emissive: Option<MaterialDef>,

    pub skins: Vec<Skin>
}

#[derive(Debug, Clone)]
pub struct IniToken<T> {
    pub range: Range<usize>, 
    pub value: T
}

pub type IniTokenPath = IniToken<PathBuf>;
pub type IniTokenTexture = IniToken<Texture>;

#[derive(Debug, Clone)]
pub enum RenderConfig<'stock> {
    Stock { key: &'stock str, data: &'stock str },
    Mod(PathBuf)
}

#[derive(Debug, Clone)]
pub struct ModelDef {
    pub ini_token: IniTokenPath,
    pub patch: Option<ModelPatch>,
}

#[derive(Debug, Clone)]
pub enum ModelPatch {
    Keep(Vec<String>),
    Remove(Vec<String>)
}

#[derive(Debug, Clone)]
pub struct MaterialDef {
    pub render_token: IniTokenPath,
    pub textures: Vec<IniTokenTexture>
}

#[derive(Debug, Clone)]
pub struct Texture {
    pub num: char,
    pub path: PathBuf
}

#[derive(Debug, Clone)]
pub struct Skin {
   pub material: SkinMaterial,
   pub material_emissive: Option<SkinMaterial>
}

#[derive(Debug, Clone)]
pub struct SkinMaterial {
    pub path: PathBuf,
    pub textures: Vec<IniTokenTexture>
}

#[derive(Debug, Clone)]
pub enum PathPrefix {
    Stock,
    Workshop,
    CurrentDir
}

//----------------------------------------------
//           STOCK BUILDINGS MAP

pub type StockBuildingsMap<'stock> = HashMap<&'stock str, (&'stock str, StockBuilding<'stock>)>;

#[derive(Debug)]
pub enum StockBuilding<'stock> {
    Unparsed(&'stock str),
    Parsed(BuildingDef<'stock>)
}


//--------------------------------------------------------
impl BuildingDef<'_> {
    pub fn validate(&self) {
        assert!(self.building_ini.exists());
        assert!(self.bbox.exists());
        assert!(path_option_valid(&self.fire));
        assert!(path_option_valid(&self.imagegui));

        let mtl_model = validate_modeldef(&self.model);
        let mtl_model_lod1 = self.model_lod1.as_ref().map(validate_modeldef);
        let mtl_model_lod2 = self.model_lod2.as_ref().map(validate_modeldef);
        let mtl_model_emissive = self.model_emissive.as_ref().map(validate_modeldef);

        // NOTE: DEBUG
        //println!("Model's actual use of submaterials: {:?}", mtl_model);

        // TODO: look for *.mtl <-> *.nmf mismatches for all model types

        validate_material(&self.material.render_token.value, self.material.textures.as_slice());
        if let Some(m) = &self.material_emissive {
            validate_material(&m.render_token.value, m.textures.as_slice());
        }

        for skin in self.skins.iter() {
            validate_material(&skin.material.path, skin.material.textures.as_slice());
            if let Some(m) = &skin.material_emissive {
                validate_material(&m.path, m.textures.as_slice());
            }
        }

        //------------------------------------
        fn validate_material(pathbuf: &PathBuf, txs: &[IniTokenTexture]) {
            assert!(pathbuf.exists());
            assert!(txs.len() > 0);
            for tx in txs.iter() {
                assert!(tx.value.path.exists(), "Material missing texture: \"{}\"", tx.value.path.to_str().unwrap());
            }
        }

        // TODO: this function sucks
        fn validate_modeldef(m: &ModelDef) -> Vec<String> {
            assert!(m.ini_token.value.exists());

            let buf = fs::read(&m.ini_token.value).unwrap();
            let (nmf, rest) = nmf::Nmf::parse_bytes(buf.as_slice()).expect("Failed to parse the model nmf");
            // NOTE: debug
            //println!("{}", nmf);
            assert_eq!(rest.len(), 0, "Model nmf parsed with leftovers");

            let mut used: Vec<(&nmf::SubMaterial, bool)> = nmf.submaterials.iter().zip(std::iter::repeat(false)).collect();

            #[inline]
            fn set_used<'a, 'b, T>(used: &mut Vec<(&'b nmf::SubMaterial<'a>, bool)>, objs: T)
            where T: Iterator<Item = &'b nmf::Object<'a>> {
                for obj in objs {
                    if let Some(idx) = obj.submaterial_idx {
                        used[idx as usize].1 = true;
                    }
                };
            }
            
            if let Some(ref p) = m.patch {
                match p {
                    ModelPatch::Keep(keeps) => {
                        let objs = keeps.iter()
                                        .map(|k| nmf.objects.iter()
                                                            .find(|o| k == o.name.as_str().unwrap())
                                                            .expect(&format!("ModelPatch error: cannot find object to keep {:?}", k)));
                        set_used(&mut used, objs);
                    },
                    ModelPatch::Remove(rems) => {
                        let to_keep: Vec<&nmf::Object> = 
                            nmf.objects.iter()
                                       .filter(|o| !rems.iter().any(|r| *r == o.name.as_str().unwrap()))
                                       .collect();
                        
                        // TODO: this is not good
                        let undeleted = to_keep.len() + rems.len() - nmf.objects.len();
                        if undeleted != 0 {
                            panic!("ModelPatch error: cannot find {} objects to delete", undeleted);
                        }

                        set_used(&mut used, to_keep.iter().map(|x| *x));
                    }
                }
            } else {
                let objs = nmf.objects.iter();
                set_used(&mut used, objs);
            }

            used.iter()
                .filter_map(|(sm, b)| 
                    if *b {
                        Some(sm.name.as_str().unwrap().to_string()) 
                    } else {
                        None 
                    }
                ).collect()
        }
    }
}


#[inline]
fn path_option_valid(opt: &Option<PathBuf>) -> bool {
    match opt {
        None => true,
        Some(ref p) => p.exists()
    }
}

/*
#[inline]
fn ini_token_valid(opt: &Option<IniTokenPath>) -> bool {
    match opt {
        None => true,
        Some(IniToken { value: ref p, .. }) => p.exists()
    }
}
*/

impl fmt::Display for BuildingDef<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        const INDENT: &str = "    ";

        write!(f, "{}renderconfig      {}\n", INDENT, self.render_config)?;
        write!(f, "{}building_ini      {}\n", INDENT, self.building_ini.to_str().unwrap())?;
        write!(f, "{}bbox              {}\n", INDENT, self.bbox.to_str().unwrap())?;
        write!(f, "{}fire              ",     INDENT)?;
        write_path_option_ln(f, &self.fire)?;
        write!(f, "{}imagegui          ",     INDENT)?;
        write_path_option_ln(f, &self.imagegui)?;
        write!(f, "{}model             {}\n", INDENT, &self.model)?;

        write!(f, "{}model_lod1        ",     INDENT)?;
        write_option_ln(f, &self.model_lod1)?;
        write!(f, "{}model_lod2        ",     INDENT)?;
        write_option_ln(f, &self.model_lod2)?;
        write!(f, "{}model_emissive    ",     INDENT)?;
        write_option_ln(f, &self.model_emissive)?;

        write!(f, "{}material          {}\n", INDENT, &self.material)?;
        write!(f, "{}material_emissive ",     INDENT)?;
        write_option_ln(f, &self.model_emissive)?;

        write!(f, "{}Skins: {:#?}", INDENT, self.skins)?;

        return Ok(());

        //------------------------------------------------

        #[inline]
        fn write_option_ln<T>(f: &mut fmt::Formatter, option: &Option<T>) -> fmt::Result
        where T: fmt::Display {
            match option {
                None => write!(f, "<none>\n"),
                Some(ref p) => write!(f, "{}\n", p)
            }
        }

        #[inline]
        fn write_path_option_ln(f: &mut fmt::Formatter, option: &Option<PathBuf>) -> fmt::Result {
            write_option_ln(f, &option.as_ref().map(|x| x.to_str().unwrap()))
        }
    }
}



//--------------------------------------------------------
impl fmt::Display for RenderConfig<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RenderConfig::Stock { key, .. } => write!(f, "Stock '{}'", key),
            RenderConfig::Mod(path) => write!(f, "Mod '{}'", path.to_str().unwrap())
        }
    }
}

impl<'stock> RenderConfig<'stock> {
    pub fn root_path(&self) -> &Path {
        match self {
            RenderConfig::Stock { .. } => APP_SETTINGS.path_stock.as_path(),
            RenderConfig::Mod(render_cfg_path) => render_cfg_path.parent().unwrap()
        }
    }
}

//--------------------------------------------------------
impl ModelDef {
    #[inline]
    pub fn new(ini_token: IniTokenPath) -> ModelDef {
        ModelDef { ini_token, patch: None }
    }
}

impl fmt::Display for ModelDef {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}",  self.ini_token)?;
        if let Some(ref p) = self.patch {
            write!(f, " ({})", p)
        } else { Ok(()) }
    }
}

//--------------------------------------------------------
impl fmt::Display for ModelPatch {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let v = match self {
            ModelPatch::Keep(v)   => { write!(f, "patch-keep: {{")?; v },
            ModelPatch::Remove(v) => { write!(f, "patch-remove: {{")?; v }
        };
        
        for x in v.iter() {
            write!(f, " \"{}\";", x)?;
        }

        write!(f, " }}")
    }
}

impl<'a, T> From<T> for ModelPatch 
where T: AsRef<str>
{
    fn from(text: T) -> ModelPatch {
        lazy_static! {
            static ref RX_LINES: Regex = Regex::new(r"\r?\n").unwrap();
        }

        let mut lines = RX_LINES.split(text.as_ref());
        let ptype = lines.next().expect("Cannot parse model.patch");

        let mut tokens = Vec::<String>::with_capacity(8);
        for l in lines {
            if l.len() > 0 {
                tokens.push(String::from(l));
            }
        }

        match ptype {
            "KEEP" => ModelPatch::Keep(tokens),
            "REMOVE" => ModelPatch::Remove(tokens),
            z => panic!("Unknown patch type {:?}", z)
        }
    }
} 

//--------------------------------------------------------
impl<T> From<(Range<usize>, T)> for IniToken<T> {
    #[inline]
    fn from((range, value): (Range<usize>, T)) -> IniToken<T> {
        IniToken { range, value }
    }
}

impl fmt::Display for IniTokenPath {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}..{}) {}", self.range.start, self.range.end, self.value.to_str().unwrap())
    }
}


//--------------------------------------------------------
impl MaterialDef {
    pub fn new(render_token: IniTokenPath) -> MaterialDef {
        let textures = get_texture_tokens(&render_token.value);
        MaterialDef { render_token, textures }
    }
}

impl fmt::Display for MaterialDef {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let t = &self.render_token;
        write!(f, "({}..{}) {} ({} textures)", t.range.start, t.range.end, t.value.to_str().unwrap(), self.textures.len())
        //write!(f, "({}..{}) {} (textures: {:#?})", t.range.start, t.range.end, t.value.to_str().unwrap(), self.textures)
    }
}


//--------------------------------------------------------
impl TryFrom<&str> for PathPrefix {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "~/" => Ok(Self::Stock),
            "$/" => Ok(Self::Workshop),
            "./" => Ok(Self::CurrentDir),
            p => Err(format!("Unknown path prefix '{}'", p))
        }
    }
}


//--------------------------------------------------------


pub fn get_texture_tokens(mtl_path: &Path) -> Vec<IniToken<Texture>> {
    use path_slash::PathBufExt;

    lazy_static! {
        static ref RX: Regex = Regex::new(concatcp!(r"(?m)^(\$TEXTURE(_MTL)?\s+?([012])\s+?", AppSettings::SRX_PATH, ")", AppSettings::SRX_EOL)).unwrap();
    }

    let ext = mtl_path.extension().unwrap();
    assert_eq!(ext.to_str().unwrap(), "mtl", "This function must be called only for *.mtl files"); 

    let mtl_src = fs::read_to_string(mtl_path).unwrap();

    RX.captures_iter(&mtl_src).map(move |cap| {
        let range = cap.get(1).unwrap().range();
        // NOTE: Debug
        // println!("CAPTURE: {:?}, {:?}", &range, cap.get(1).unwrap().as_str());
        let is_mtl = cap.get(2).is_some();
        let num = cap.get(3).unwrap().as_str().chars().next().unwrap();
        let tx_path_str = cap.get(4).unwrap().as_str();

        let tx_root = if is_mtl { 
            mtl_path.parent().unwrap() 
        } else {
            APP_SETTINGS.path_stock.as_path()
        };

        let path = tx_root.join(PathBuf::from_slash(tx_path_str));

        IniToken {
            range,
            value: Texture { num, path }
        }
    }).collect()
}

pub fn resolve_prefixed_path(pfx: PathPrefix, path_str: &str, local_root: &Path) -> PathBuf {
    use path_slash::PathBufExt;

    let root = match pfx {
        PathPrefix::Stock => APP_SETTINGS.path_stock.as_path(),
        PathPrefix::Workshop => APP_SETTINGS.path_workshop.as_path(),
        PathPrefix::CurrentDir => local_root,
    };

    root.join(PathBuf::from_slash(path_str))
}

pub fn read_to_string_buf(path: &Path, buf: &mut String) {
    use std::io::Read;

    if let Ok(mut file) = fs::File::open(path) {
        let meta = file.metadata().unwrap();
        let sz: usize = meta.len().try_into().unwrap();
        buf.reserve(sz);
        file.read_to_string(buf).unwrap();
    } else {
        panic!("Cannot read file \"{}\"", path.display());
    }
}
/*
fn read_to_buf(path: &Path, buf: &mut Vec<u8>) {
    use std::io::Read;

    if let Ok(mut file) = fs::File::open(path) {
        let meta = file.metadata().unwrap();
        let sz: usize = meta.len().try_into().unwrap();
        buf.reserve(sz);
        file.read(buf).unwrap();
    } else {
        panic!("Cannot read file \"{}\"", path.display());
    }
}
*/