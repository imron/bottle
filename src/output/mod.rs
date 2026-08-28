mod render;
mod tsv;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    Tsv,
    Yaml,
}

pub use render::render;
