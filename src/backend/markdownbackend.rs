use std::fs::File;
use std::io::Write;

use crate::backend::Backend;
use crate::parser::{DocumentationData, DocumentationEntry};
use crate::parser::{ExportArgStruct, FunctionArgStruct, SymbolArgs, VariableArgStruct};

use std::fmt::Display;

pub struct MarkdownBackend {}

impl MarkdownBackend {
    pub fn new() -> MarkdownBackend {
        MarkdownBackend {}
    }
}

fn sanitize_markdown(s: String) -> String {
    s.replace("\\", "\\\\")
        .replace("_", "\\_")
        .replace("#", "\\#")
        .replace("*", "\\*")
        .replace("`", "\\`")
        .replace("(", "\\(")
        .replace(")", "\\)")
        .replace("[", "\\[")
        .replace("]", "\\]")
}

fn sanitize_markdown_quoted(s: String) -> String {
    s.replace("*", "\\*").replace("`", "\\`")
}

fn sanitize_markdown_format(f: &impl Display) -> String {
    sanitize_markdown(format!("{}", f))
}

fn format_comments(prefix: &String, text: Vec<String>) -> String {
    if text.is_empty() {
        return "  \n".to_string();
    }

    format!(
        "  \n{prefix}    ```\n{prefix}    {}\n{prefix}    ```\n\n",
        text.join(format!("\n{}    ", prefix).as_str()),
        prefix = prefix,
    )
}

fn join<T: Display>(v: Vec<T>, s: &str) -> String {
    v.iter()
        .map(|x| sanitize_markdown_format(x))
        .collect::<Vec<_>>()
        .join(s)
}

fn write_symbols(
    prefix: String,
    entries: Vec<DocumentationEntry>,
    f: &mut File,
) -> std::io::Result<()> {
    for entry in entries {
        write!(f, "{}* **{}**:  \n", prefix, entry.entry_type)?;

        for entry in entry.symbols {
            let sanitized_name = sanitize_markdown(entry.name);

            write!(f, "{}    * {}", prefix, sanitized_name)?;
            if let Some(args) = entry.arg {
                match args {
                    SymbolArgs::FunctionArgs(FunctionArgStruct {
                        arguments,
                        super_arguments,
                        return_type,
                    }) => {
                        write!(f, "({})", join(arguments, ", "))?;
                        if let Some(return_type) = return_type {
                            write!(f, " -> {}", sanitize_markdown(return_type))?;
                        }
                        if let Some(super_arguments) = super_arguments {
                            write!(
                                f,
                                "  \n{}**Calls**: super.{}({})",
                                prefix,
                                sanitized_name,
                                join(super_arguments, ", ")
                            )?;
                        }
                    }
                    SymbolArgs::VariableArgs(VariableArgStruct {
                        value_type,
                        assignment,
                        setter,
                        getter,
                    }) => {
                        if let Some(value_type) = value_type {
                            write!(f, ": {}", sanitize_markdown(value_type))?;
                        }
                        if let Some(assignment) = assignment {
                            write!(f, " = `{}`", sanitize_markdown_quoted(assignment))?;
                        }
                        if let Some(getter) = getter {
                            write!(f, "  \n{}**Getter**: {}", prefix, sanitize_markdown(getter))?;
                        }
                        if let Some(setter) = setter {
                            write!(f, "  \n{}**Setter**: {}", prefix, sanitize_markdown(setter))?;
                        }
                    }
                    SymbolArgs::ExportArgs(ExportArgStruct {
                        value_type,
                        assignment,
                        options,
                        setter,
                        getter,
                    }) => {
                        if let Some(value_type) = value_type {
                            if options.len() == 0 {
                                write!(f, ": {}", sanitize_markdown(value_type))?;
                            } else {
                                write!(
                                    f,
                                    ": ({}, {})",
                                    sanitize_markdown(value_type),
                                    sanitize_markdown(options.join(", "))
                                )?;
                            }
                        }
                        if let Some(assignment) = assignment {
                            write!(f, " = `{}`", sanitize_markdown_quoted(assignment))?;
                        }
                        if let Some(getter) = getter {
                            write!(f, "  \n{}**Getter**: {}", prefix, sanitize_markdown(getter))?;
                        }
                        if let Some(setter) = setter {
                            write!(f, "  \n{}**Setter**: {}", prefix, sanitize_markdown(setter))?;
                        }
                    }
                    SymbolArgs::EnumArgs(values) => {
                        write!(f, "  \n{}    **Values**:", prefix)?;
                        for val in values {
                            write!(
                                f,
                                "  \n{}    * {} = {}",
                                prefix,
                                sanitize_markdown(val.name),
                                val.value
                            )?;
                            write!(f, "{}", format_comments(&prefix, val.text))?;
                        }
                    }
                    SymbolArgs::ClassArgs(entries) => {
                        write!(f, "{}", format_comments(&prefix, entry.text))?;
                        write_symbols(format!("{}{}", prefix, "        "), entries, f)?;
                        continue;
                    }
                }
            }
            write!(f, "{}", format_comments(&prefix, entry.text))?;
        }
    }

    Ok(())
}

impl Backend for MarkdownBackend {
    fn get_extension(&self) -> String {
        "md".to_string()
    }

    fn generate_output(&self, data: DocumentationData, f: &mut File) -> std::io::Result<()> {
        write!(f, "## {}\n\n", sanitize_markdown(data.source_file))?;

        for entry in data.entries {
            write!(f, "### {}:  \n", entry.entry_type)?;

            for entry in entry.symbols {
                let sanitized_name = sanitize_markdown(entry.name);

                write!(f, "* {}", sanitized_name)?;
                if let Some(args) = entry.arg {
                    match args {
                        SymbolArgs::FunctionArgs(FunctionArgStruct {
                            arguments,
                            super_arguments,
                            return_type,
                        }) => {
                            write!(f, "({})", join(arguments, ", "))?;
                            if let Some(return_type) = return_type {
                                write!(f, " -> {}", sanitize_markdown(return_type))?;
                            }
                            if let Some(super_arguments) = super_arguments {
                                write!(
                                    f,
                                    "  \n**Calls**: super.{}({})",
                                    sanitized_name,
                                    join(super_arguments, ", ")
                                )?;
                            }
                        }
                        SymbolArgs::VariableArgs(VariableArgStruct {
                            value_type,
                            assignment,
                            setter,
                            getter,
                        }) => {
                            if let Some(value_type) = value_type {
                                write!(f, ": {}", sanitize_markdown(value_type))?;
                            }
                            if let Some(assignment) = assignment {
                                write!(f, " = `{}`", sanitize_markdown_quoted(assignment))?;
                            }
                            if let Some(getter) = getter {
                                write!(f, "  \n**Getter**: {}", sanitize_markdown(getter))?;
                            }
                            if let Some(setter) = setter {
                                write!(f, "  \n**Setter**: {}", sanitize_markdown(setter))?;
                            }
                        }
                        SymbolArgs::ExportArgs(ExportArgStruct {
                            value_type,
                            assignment,
                            options,
                            setter,
                            getter,
                        }) => {
                            if let Some(value_type) = value_type {
                                if options.len() == 0 {
                                    write!(f, ": {}", sanitize_markdown(value_type))?;
                                } else {
                                    write!(
                                        f,
                                        ": ({}, {})",
                                        sanitize_markdown(value_type),
                                        sanitize_markdown(options.join(", "))
                                    )?;
                                }
                            }
                            if let Some(assignment) = assignment {
                                write!(f, " = `{}`", sanitize_markdown_quoted(assignment))?;
                            }
                            if let Some(getter) = getter {
                                write!(f, "  \n**Getter**: {}", sanitize_markdown(getter))?;
                            }
                            if let Some(setter) = setter {
                                write!(f, "  \n**Setter**: {}", sanitize_markdown(setter))?;
                            }
                        }
                        SymbolArgs::EnumArgs(values) => {
                            write!(f, "  \n    **Values**:")?;
                            for val in values {
                                write!(
                                    f,
                                    "  \n    * {} = {}",
                                    sanitize_markdown(val.name),
                                    val.value
                                )?;
                                if !val.text.is_empty() {
                                    write!(
                                        f,
                                        "  \n    {}",
                                        format_comments(&"".to_string(), val.text)
                                    )?;
                                }
                            }
                        }
                        SymbolArgs::ClassArgs(entries) => {
                            write!(
                                f,
                                "  \n{}  \n",
                                format_comments(&"".to_string(), entry.text)
                            )?;
                            write_symbols("    ".to_string(), entries, f)?;
                            continue;
                        }
                    }
                }
                write!(f, "  \n{}", format_comments(&"".to_string(), entry.text))?;
            }
            write!(f, "  \n")?;
        }

        Ok(())
    }
}
