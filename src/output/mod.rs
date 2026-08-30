mod render;
mod tsv;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    Tsv,
    TsvNoHeader,
    Yaml,
}

pub use render::render;
