use std::io::Write;
use std::fmt;

pub mod common;

pub mod building;
pub mod renderconfig;
pub mod material;

pub mod transform;

use common::ParseError;


//---------------------------------------------


pub trait IniToken: Sized {
    fn serialize<W: Write>(&self, wr: W) -> std::io::Result<()>;
}


pub enum IniTokenState<T> {
    Original(T),
    Modified(T)
}


impl<T> IniTokenState<T> {
    pub fn token(&self) -> &T {
        match self {
            Self::Original(t) => t,
            Self::Modified(t) => t
        }
    }

    pub fn modify<F: FnMut(&T) -> Option<T>>(&mut self, mut f: F) {
        match f(self.token()) {
            None => { },
            Some(t) => *self = Self::Modified(t)
        }
    }
}


impl<T: fmt::Display> fmt::Display for IniTokenState<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            Self::Original(t) => write!(f, "{}", t),
            Self::Modified(t) => write!(f, "* {}", t),
        }
    }
}


//-------------------------------------------------
pub trait Captures<'a> {}
impl<'a, T: ?Sized> Captures<'a> for T {}

pub struct IniFile<'a, T: IniToken> {
    ini_slice: &'a str,
    tokens: Vec<(&'a str, IniTokenState<T>)>
}


impl<'a, T> IniFile<'a, T> where T: IniToken {

    pub fn from_parts(ini_slice: &'a str, tokens: Vec<(&'a str, T)>) -> Self {
        IniFile { 
            ini_slice, 
            tokens: tokens.into_iter()
                          .map(|(chunk, t_val)| (chunk, IniTokenState::Original(t_val)))
                          .collect()
        }
    }

    pub fn tokens(&self) -> impl Iterator<Item = &T> {
        self.tokens.iter().map(|(_, t)| t.token())
    }

    pub fn tokens_mut(&mut self) -> impl Iterator<Item = &mut IniTokenState<T>> + Captures<'a> {
        self.tokens.iter_mut().map(|(_, t)| t)
    }

    pub fn write_file<P: AsRef<std::path::Path>>(&self, path: P) -> std::io::Result<()> {
        let mut new_ini_file = std::io::BufWriter::new(std::fs::OpenOptions::new().write(true).create(true).truncate(true).open(path)?);
        self.write_to(&mut new_ini_file)?;
        new_ini_file.flush()
    }

    pub fn write_to<W: Write>(&self, mut wr: W) -> std::io::Result<()> {
        unsafe {
            // replace 'modified' tokens, dump other stuff as is
            let mut chunk_start = self.ini_slice.as_ptr();

            for (t_str, t_state) in self.tokens.iter() {
                match t_state {
                    IniTokenState::Original(_) => { },
                    IniTokenState::Modified(t) => {
                        let chunk_end = t_str.as_ptr();
                        let chunk_len = chunk_end.offset_from(chunk_start);
                        if chunk_len > 0 {
                            // can write original chunk
                            let chunk = std::slice::from_raw_parts(chunk_start, chunk_len as usize);
                            wr.write_all(chunk)?;

                            chunk_start = chunk_end.add(t_str.len());
                        }

                        t.serialize(&mut wr)?;
                    }
                }
            }

            let written = chunk_start.offset_from(self.ini_slice.as_ptr());
            assert!(written >= 0);
            let written = written as usize;
            assert!(written <= self.ini_slice.len());
            let remaining = self.ini_slice.len() - written;
            if remaining > 0 {
                let last_chunk = std::slice::from_raw_parts(chunk_start, remaining);
                wr.write_all(last_chunk)?;
            }
        } //unsafe

        Ok(())
    }
}


pub type BuildingToken<'a> = building::Token<'a>;
pub type BuildingIni<'a> = IniFile<'a, BuildingToken<'a>>;
pub use building::parse_tokens as parse_building_tokens;

pub fn parse_building_ini<'a>(src: &'a str) -> Result<BuildingIni<'a>, Vec<(&'a str, ParseError)>> {
    building::parse_tokens_strict(src).map(|tokens| BuildingIni::from_parts(src, tokens))
}


pub type RenderToken<'a> = renderconfig::Token<'a>;
pub type RenderIni<'a> = IniFile<'a, RenderToken<'a>>;
pub use renderconfig::parse_tokens as parse_render_tokens;

pub fn parse_renderconfig_ini<'a>(src: &'a str) -> Result<RenderIni<'a>, Vec<(&'a str, ParseError)>> {
    renderconfig::parse_tokens_strict(src).map(|tokens| RenderIni::from_parts(src, tokens))
}


pub type MaterialToken<'a> = material::Token<'a>;
pub type MaterialMtl<'a> = IniFile<'a, MaterialToken<'a>>;
pub use material::parse_tokens as parse_material_tokens;

pub fn parse_mtl<'a>(src: &'a str) -> Result<MaterialMtl<'a>, Vec<(&'a str, ParseError)>> {
    material::parse_tokens_strict(src).map(|tokens| MaterialMtl::from_parts(src, tokens))
}

use std::path::PathBuf;

impl MaterialMtl<'_> {
    pub fn get_texture_paths<F: Fn(&str) -> PathBuf>(&self, path_resolver: F) -> Vec<PathBuf> {
        use crate::cfg::APP_SETTINGS;
        use crate::ini::MaterialToken as MT;

        self.tokens().filter_map(|t| match t {
            MT::Texture((_, s))         => Some(APP_SETTINGS.path_stock.join(s.as_str()).into_path_buf()),
            MT::TextureNoMip((_, s))    => Some(APP_SETTINGS.path_stock.join(s.as_str()).into_path_buf()),
            MT::TextureMtl((_, s))      => Some(path_resolver(s.as_str())),
            MT::TextureNoMipMtl((_, s)) => Some(path_resolver(s.as_str())),
            _ => None
        }).collect()
    }
}
