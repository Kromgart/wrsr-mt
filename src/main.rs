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

mod input;
mod output;


//--------------------------------------------
//                SOURCES

#[derive(Debug, Clone)]
struct BuildingDef<'stock> {
    render_config: RenderConfig<'stock>,

    building_ini: PathBuf,
    bbox: PathBuf,
    fire: Option<PathBuf>,
    imagegui: Option<PathBuf>,

    model: IniTokenPath,
    model_lod1: Option<IniTokenPath>,
    model_lod2: Option<IniTokenPath>,
    model_emissive: Option<IniTokenPath>,

    material: MaterialDef,
    material_emissive: Option<MaterialDef>,

    skins: Vec<Skin>
}

#[derive(Debug, Clone)]
enum RenderConfig<'stock> {
    Stock { key: &'stock str, data: &'stock str },
    Mod(PathBuf)
}

#[derive(Debug, Clone)]
struct IniToken<T> {
    range: Range<usize>, 
    value: T
}

type IniTokenPath = IniToken<PathBuf>;
type IniTokenTexture = IniToken<Texture>;

#[derive(Debug, Clone)]
struct MaterialDef {
    render_token: IniTokenPath,
    textures: Vec<IniTokenTexture>
}

#[derive(Debug, Clone)]
struct Texture {
    num: char,
    path: PathBuf
}

#[derive(Debug, Clone)]
struct Skin {
   material: SkinMaterial,
   material_emissive: Option<SkinMaterial>
}

#[derive(Debug, Clone)]
struct SkinMaterial {
    path: PathBuf,
    textures: Vec<IniTokenTexture>
}

#[derive(Debug, Clone)]
enum PathPrefix {
    Stock,
    Workshop,
    CurrentDir
}

//----------------------------------------------
//           STOCK BUILDINGS MAP

type StockBuildingsMap<'stock> = HashMap<&'stock str, (&'stock str, StockBuilding<'stock>)>;

#[derive(Debug)]
enum StockBuilding<'stock> {
    Unparsed(&'stock str),
    Parsed(BuildingDef<'stock>)
}


//-----------------------------------------------
//              Global constants

// mod folder is 7 digits and cannot start from zero.
const MOD_IDS_START: u32 = 1_000_000;
const MOD_IDS_END: u32 = 9_999_999;
const MAX_MODS: usize = (MOD_IDS_END - MOD_IDS_START) as usize;

const MAX_BUILDINGS_IN_MOD: u8 = 99;

const MAX_BUILDINGS: usize = MAX_MODS * (MAX_BUILDINGS_IN_MOD as usize);

// TODO: check this.
const MAX_SKINS_IN_MOD: u8 = 99;

// Paths in ini files:
const SRX_PATH_PREFIX: &str = "([~.$]/)";
const SRX_PATH: &str = r"([^\r\s\n]+?)";
const SRX_PATH_EXT: &str = "([^\"\\r\\n]+?)";
const SRX_EOL: &str = r"(:?[\s\r\n$])";


lazy_static! {
    static ref ARGS: Vec<String> = std::env::args().collect();
    static ref PATH_ROOT_STOCK: PathBuf = get_path_arg_or(3, r"C:\Program Files (x86)\Steam\steamapps\common\SovietRepublic\media_soviet");
    static ref PATH_ROOT_MODS:  PathBuf = get_path_arg_or(4, r"C:\Program Files (x86)\Steam\steamapps\workshop\content\784150");
}

fn get_path_arg_or(idx: usize, default: &str) -> PathBuf {
    if let Some(p) = ARGS.get(idx) {
        PathBuf::from(p)
    } else {
        PathBuf::from(default)
    }
}



// ------------------------------------------------------------------

fn main() {
    let src = {
        let mut src = PathBuf::from(std::env::current_dir().unwrap());
        src.push(ARGS.get(1).map(String::as_str).unwrap_or("."));
        src
    };

    let dest = get_path_arg_or(2, r"C:\Program Files (x86)\Steam\steamapps\common\SovietRepublic\media_soviet\workshop_wip");

    println!("Pack source:      {}", src.to_str().unwrap());
    println!("Installing to:    {}", dest.to_str().unwrap());
    println!("Stock game files: {}", PATH_ROOT_STOCK.to_str().unwrap());
    println!("Mod files:        {}", PATH_ROOT_MODS.to_str().unwrap());

    let mut pathbuf: PathBuf = [PATH_ROOT_STOCK.as_os_str(), "buildings".as_ref(), "buildingtypes.ini".as_ref()].iter().collect();

    let stock_buildings_ini = fs::read_to_string(&pathbuf).unwrap();
    let mut stock_buildings = { 
        let mut mp = HashMap::with_capacity(512);
        let rx = Regex::new(r"\$TYPE ([_[:alnum:]]+?)\r\n((?s).+?\n END\r\n)").unwrap();

        for caps in rx.captures_iter(&stock_buildings_ini) {
            let key = caps.get(1).unwrap().as_str();
            mp.insert(
                key, 
                (key, StockBuilding::Unparsed(caps.get(2).unwrap().as_str()))
            );
        }
        
        mp
    };

    println!("Found {} stock buildings", stock_buildings.len());

    pathbuf.push(src);
    println!("Reading sources...");
    let data = input::read_validate_sources(pathbuf.as_path(), &mut stock_buildings);
    println!("Sources verified.");


    println!("Creating mods...");
    pathbuf.push(dest);

    output::generate_mods(pathbuf.as_path(), data);

}







//--------------------------------------------------------
impl BuildingDef<'_> {
    fn validate_paths(&self) {
        assert!(self.building_ini.exists());
        
        assert!(self.bbox.exists());
        assert!(path_option_valid(&self.fire));
        assert!(path_option_valid(&self.imagegui));

        assert!(self.model.value.exists());
        assert!(ini_token_valid(&self.model_lod1));
        assert!(ini_token_valid(&self.model_lod2));
        assert!(ini_token_valid(&self.model_emissive));

        validate_material(&self.material);
        if let Some(m) = &self.material_emissive {
            validate_material(m);
        }

        for skin in self.skins.iter() {
            validate_skin_material(&skin.material);
            if let Some(m) = &skin.material_emissive {
                validate_skin_material(m);
            }
        }

        //------------------------------------
        #[inline]
        fn validate_material(m: &MaterialDef) {
            assert!(m.render_token.value.exists());
            assert!(m.textures.len() > 0);
            for tx in m.textures.iter() {
                assert!(tx.value.path.exists(), "Material missing texture: \"{}\"", tx.value.path.to_str().unwrap());
            }
        }

        #[inline]
        fn validate_skin_material(m: &SkinMaterial) {
            assert!(m.path.exists());
            assert!(m.textures.len() > 0);
            for tx in m.textures.iter() {
                assert!(tx.value.path.exists(), "Emissive material missing texture: \"{}\"", tx.value.path.to_str().unwrap());
            }
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


#[inline]
fn ini_token_valid(opt: &Option<IniTokenPath>) -> bool {
    match opt {
        None => true,
        Some(IniToken { value: ref p, .. }) => p.exists()
    }
}

// TODO: remove Strings cruft

impl fmt::Display for BuildingDef<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        //write!(f, "\ //"BuildingDef {{\n\
        write!(f, "\
        {indent}renderconfig      {}\n\
        {indent}building_ini      {}\n\
        {indent}bbox              {}\n\
        {indent}fire              {}\n\
        {indent}imagegui          {}\n\
        {indent}model             {}\n\
        {indent}model_lod1        {}\n\
        {indent}model_lod2        {}\n\
        {indent}model_emissive    {}\n\
        {indent}material          {}\n\
        {indent}material_emissive {}\n\
        ", //}}",
        self.render_config, 
        self.building_ini.to_str().unwrap(),
        self.bbox.to_str().unwrap(),
        print_path_option(&self.fire),
        print_path_option(&self.imagegui),
        self.model,
        print_displayed_option(&self.model_lod1),
        print_displayed_option(&self.model_lod2),
        print_displayed_option(&self.model_emissive),
        self.material,
        print_option(&self.material_emissive, |x| x.to_string()),
        indent = "   " 
        )?;

        write!(f, "Skins: {:#?}", self.skins)
    }
}

#[inline]
fn print_path_option(o: &Option<PathBuf>) -> String {
    print_option(o, |x| String::from(x.to_str().unwrap()))
}

#[inline]
fn print_displayed_option<T: fmt::Display>(o: &Option<T>) -> String {
    print_option(o, T::to_string)
}

#[inline]
fn print_option<T, F: Fn(&T) -> String>(o: &Option<T>, f: F) -> String {
    match o {
        None => String::from("<none>"),
        Some(ref p) => f(p)
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
    fn root_path(&self) -> &Path {
        match self {
            RenderConfig::Stock { .. } => PATH_ROOT_STOCK.as_path(),
            RenderConfig::Mod(render_cfg_path) => render_cfg_path.parent().unwrap()
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
    fn new(render_token: IniTokenPath) -> MaterialDef {
        let textures = get_texture_tokens(&render_token.value);
        MaterialDef { render_token, textures }
    }
}

impl fmt::Display for MaterialDef {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let t = &self.render_token;
        write!(f, "({}..{}) {} (contains {} textures)", t.range.start, t.range.end, t.value.to_str().unwrap(), self.textures.len())
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


fn get_texture_tokens(mtl_path: &Path) -> Vec<IniToken<Texture>> {
    use path_slash::PathBufExt;

    lazy_static! {
        static ref RX: Regex = Regex::new(concatcp!(r"(?m)^(\$TEXTURE(_MTL)?\s+?([012])\s+?", SRX_PATH, ")", SRX_EOL)).unwrap();
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
            PATH_ROOT_STOCK.as_path()
        };

        let path = tx_root.join(PathBuf::from_slash(tx_path_str));

        IniToken {
            range,
            value: Texture { num, path }
        }
    }).collect()
}

fn resolve_prefixed_path(pfx: PathPrefix, path_str: &str, local_root: &Path) -> PathBuf {
    use path_slash::PathBufExt;

    let root = match pfx {
        PathPrefix::Stock => PATH_ROOT_STOCK.as_path(),
        PathPrefix::Workshop => PATH_ROOT_MODS.as_path(),
        PathPrefix::CurrentDir => local_root,
    };

    root.join(PathBuf::from_slash(path_str))
}

fn read_to_string_buf(path: &Path, buf: &mut String) {
    use std::io::Read;

    let mut file = fs::File::open(path).unwrap();
    let meta = file.metadata().unwrap();
    let sz: usize = meta.len().try_into().unwrap();
    buf.reserve(sz);
    file.read_to_string(buf).unwrap();
}
