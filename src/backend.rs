use crate::parser::DocumentationData;

use std::fs::File;

pub trait Backend {
    fn generate_output(&self, data: DocumentationData, f: &mut File) -> std::io::Result<()>;
    fn get_extension(&self) -> String;
}

pub mod markdownbackend;
