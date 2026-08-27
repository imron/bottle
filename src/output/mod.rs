mod render;
mod tsv;

pub use render::render;

#[derive(Debug, Clone, Copy)]
pub enum Style {
    Tsv,
    Yaml,
}
