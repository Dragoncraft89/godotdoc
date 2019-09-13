use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Lines;
use std::io::Read;

use std::fmt::{Display, Formatter};

use crate::Settings;

pub enum EntryType {
    CLASS,
    SIGNAL,
    FUNC,
    VAR,
    CONST,
    EXPORT,
    ENUM,
}

impl Display for EntryType {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            EntryType::CLASS => write!(f, "Classes"),
            EntryType::SIGNAL => write!(f, "Signals"),
            EntryType::FUNC => write!(f, "Functions"),
            EntryType::VAR => write!(f, "Variables"),
            EntryType::CONST => write!(f, "Constants"),
            EntryType::EXPORT => write!(f, "Exports"),
            EntryType::ENUM => write!(f, "Enums"),
        }
    }
}

pub struct FunctionArgument {
    pub name: String,
    pub value_type: Option<String>,
    pub default_value: Option<String>,
}

impl Display for FunctionArgument {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", self.name)?;
        if self.value_type.is_some() {
            write!(f, ": {}", self.value_type.as_ref().unwrap())?;
        }
        if self.default_value.is_some() {
            write!(f, " = {}", self.default_value.as_ref().unwrap())?;
        }

        Ok(())
    }
}

pub struct FunctionArgStruct {
    pub arguments: Vec<FunctionArgument>,
    pub super_arguments: Option<Vec<FunctionArgument>>,
    pub return_type: Option<String>,
}

pub struct VariableArgStruct {
    pub value_type: Option<String>,
    pub assignment: Option<String>,
    pub setter: Option<String>,
    pub getter: Option<String>,
}

pub struct ExportArgStruct {
    pub value_type: Option<String>,
    pub assignment: Option<String>,
    pub options: Vec<String>,
    pub setter: Option<String>,
    pub getter: Option<String>,
}

pub struct EnumValue {
    pub name: String,
    pub value: isize,
    pub text: Vec<String>,
}

pub enum SymbolArgs {
    FunctionArgs(FunctionArgStruct),
    VariableArgs(VariableArgStruct),
    ExportArgs(ExportArgStruct),
    EnumArgs(Vec<EnumValue>),
    ClassArgs(Vec<DocumentationEntry>),
}

pub struct Symbol {
    pub name: String,
    pub args: Option<SymbolArgs>,
    pub text: Vec<String>,
}

pub struct DocumentationEntry {
    pub entry_type: EntryType,
    pub symbols: Vec<Symbol>,
}

pub struct DocumentationData {
    pub source_file: String,
    pub entries: Vec<DocumentationEntry>,
}

struct FileIterator<R: Read> {
    reader: Lines<BufReader<R>>,
    lineno: u32
}

impl<R: Read> FileIterator<R> {
    fn new(r: R) -> FileIterator<R> {
        FileIterator {
            reader: BufReader::new(r).lines(),
            lineno: 0
        }
    }

    fn lineno(&self) -> u32 {
        self.lineno
    }
}

impl<R: Read> Iterator for FileIterator<R> {
    type Item = Result<String, String>;

    fn next(&mut self) -> Option<Result<String, String>> {
        if let Some(x) = self.reader.next() {
            self.lineno += 1;
            return Some(x.map_err(|e| e.to_string()));
        }
        None
    }
}

fn get_indentation_level(s: &str) -> u32 {
    let mut i = 0;
    for c in s.chars() {
        if c != '\t' {
            return i;
        }
        i += 1;
    }

    return i;
}

#[derive(Default)]
struct ClassFrame {
    classes: Vec<Symbol>,
    signals: Vec<Symbol>,
    functions: Vec<Symbol>,
    variables: Vec<Symbol>,
    constants: Vec<Symbol>,
    exports: Vec<Symbol>,
    enums: Vec<Symbol>,
}

#[derive(Default)]
struct EnumFrame {
    last_value: isize,
    values: Vec<EnumValue>,
}

enum Mode {
    Normal(ClassFrame),
    Enum(String, EnumFrame),
    Class(String, (u32, Option<u32>), ClassFrame, Vec<String>),
}

pub fn parse_file(
    filename: &str,
    f: File,
    settings: &Settings,
) -> Result<DocumentationData, String> {
    let mut parsing_mode = vec![Mode::Normal(ClassFrame::default())];

    let mut comment_buffer: Vec<String> = Vec::new();
    let mut override_visibility = None;
    let mut open_parentheses = Vec::new();

    let mut lines = FileIterator::new(f);
    while let Some(line) = lines.next() {
        let mut l = line?;
        while l.ends_with("\\") && !l.contains('#') {
            l.remove(l.len() - 1);
            if let Some(line) = lines.next() {
                l += line?.as_str();
            } else {
                return Err("Unexpected eof, expected newline after \\".to_string());
            }
        }

        let mut line = l.as_str();
        let indentation_level = get_indentation_level(line);

        let (pos, v) = find(filename, lines.lineno(), line, '#', open_parentheses)?;
        open_parentheses = v;
        if let Some(pos) = pos {
            let comment = line[pos + 1..].trim();
            if comment == "[Show]" {
                override_visibility = Some(true);
            } else if comment == "[Hide]" {
                override_visibility = Some(false);
            }
            if !comment.starts_with("warning-ignore:") {
                comment_buffer.push(comment.to_string());
            }
            line = &line[..pos];
        }

        match parsing_mode.last_mut().unwrap() {
            Mode::Enum(ref name, ref mut enum_frame) => {
                let end = line.find('}');
                let slice = match end {
                    Some(x) => &line[..x],
                    None => &line,
                };
                for v in slice.split(',') {
                    let mut arg_iterator = v.split('=');

                    let name = arg_iterator.next().unwrap().trim().to_string();
                    if name.is_empty() {
                        continue;
                    }
                    let value = arg_iterator
                        .next()
                        .and_then(|x| x.trim().parse().ok())
                        .unwrap_or(enum_frame.last_value);

                    enum_frame.last_value = value + 1;

                    if (!name.starts_with("_") || settings.show_prefixed)
                        && override_visibility.unwrap_or(true)
                    {
                        enum_frame.values.push(EnumValue {
                            name: name,
                            value: value,
                            text: comment_buffer.drain(..).collect(),
                        });
                    }
                }

                if end.is_some() {
                    let name_string = name.to_string();
                    let values = enum_frame.values.drain(..).collect();
                    parsing_mode.pop();
                    match parsing_mode.last_mut() {
                        Some(Mode::Normal(ref mut frame))
                        | Some(Mode::Class(_, _, ref mut frame, _)) => frame.enums.push(Symbol {
                            name: name_string,
                            args: Some(SymbolArgs::EnumArgs(values)),
                            text: comment_buffer.drain(..).collect(),
                        }),
                        Some(Mode::Enum(_, _)) => {
                            panic!("[parser.rs] Unexpected Enum value after completed enum")
                        }
                        None => panic!("[parser.rs] Unexpected end of parsing_mode stack"),
                    }
                }
            }

            Mode::Class(_, (ref old_indent, ref mut indent), ref mut frame, _) => {
                if line.trim().is_empty() {
                    continue;
                }
                if indent.is_none() {
                    if indentation_level > *old_indent {
                        *indent = Some(indentation_level);
                    } else {
                        return Err(format!(
                            "Failed to parse {}, line {}: Indented block expected",
                            filename, lines.lineno()
                        ));
                    }
                }
                let indent = indent.unwrap();
                if indentation_level == indent {
                    if let Some(m) = parse_class_content(
                        filename,
                        lines.lineno(),
                        &line.trim(),
                        indentation_level,
                        frame,
                        &mut comment_buffer,
                        settings,
                        &mut override_visibility,
                    )? {
                        parsing_mode.push(m);
                    }
                } else if indentation_level < indent {
                    let mut entries = Vec::new();
                    let comments;
                    let class_name;
                    if let Mode::Class(name, _, frame, text) = parsing_mode.pop().unwrap() {
                        class_name = name;
                        add_entries(&mut entries, frame);

                        comments = text;
                    } else {
                        panic!()
                    }

                    match parsing_mode.last_mut() {
                        Some(Mode::Normal(ref mut frame))
                        | Some(Mode::Class(_, _, ref mut frame, _)) => frame.classes.push(Symbol {
                            name: class_name,
                            args: Some(SymbolArgs::ClassArgs(entries)),
                            text: comments,
                        }),
                        Some(Mode::Enum(_, _)) => {
                            panic!("[parser.rs] Unexpected Enum value after completed class")
                        }
                        None => panic!("[parser.rs] Unexpected end of parsing_mode stack"),
                    }
                }
            }

            Mode::Normal(ref mut frame) => {
                if let Some(new_frame) = parse_class_content(
                    filename,
                    lines.lineno(),
                    line,
                    indentation_level,
                    frame,
                    &mut comment_buffer,
                    settings,
                    &mut override_visibility,
                )? {
                    parsing_mode.push(new_frame);
                }
            }
        }
        if !line.is_empty() {
            comment_buffer.clear();
            override_visibility = None;
        }
    }

    while parsing_mode.len() > 0 {
        match parsing_mode.pop().unwrap() {
            Mode::Class(name, _, frame, text) => {
                let class_name = name;
                let mut entries = Vec::new();
                add_entries(&mut entries, frame);

                let comments = text;
                match parsing_mode.last_mut() {
                    Some(Mode::Normal(ref mut frame))
                    | Some(Mode::Class(_, _, ref mut frame, _)) => frame.classes.push(Symbol {
                        name: class_name,
                        args: Some(SymbolArgs::ClassArgs(entries)),
                        text: comments,
                    }),
                    Some(Mode::Enum(_, _)) => {
                        panic!("[parser.rs] Unexpected Enum value after completed class")
                    }
                    None => panic!("[parser.rs] Unexpected end of parsing_mode stack"),
                }
            }
            Mode::Enum(name, enum_frame) => {
                let name_string = name.to_string();
                let values = enum_frame.values;
                match parsing_mode.last_mut() {
                    Some(Mode::Normal(ref mut frame))
                    | Some(Mode::Class(_, _, ref mut frame, _)) => frame.enums.push(Symbol {
                        name: name_string,
                        args: Some(SymbolArgs::EnumArgs(values)),
                        text: comment_buffer.drain(..).collect(),
                    }),
                    Some(Mode::Enum(_, _)) => {
                        panic!("[parser.rs] Unexpected Enum value after completed enum")
                    }
                    None => panic!("[parser.rs] Unexpected end of parsing_mode stack"),
                }
            }

            Mode::Normal(frame) => {
                let mut entries = Vec::new();
                add_entries(&mut entries, frame);

                return Ok(DocumentationData {
                    source_file: filename.to_string(),
                    entries: entries,
                });
            }
        }
    }

    panic!()
}

fn add_entries(entries: &mut Vec<DocumentationEntry>, frame: ClassFrame) {
    if !frame.classes.is_empty() {
        entries.push(DocumentationEntry {
            entry_type: EntryType::CLASS,
            symbols: frame.classes,
        })
    }
    if !frame.enums.is_empty() {
        entries.push(DocumentationEntry {
            entry_type: EntryType::ENUM,
            symbols: frame.enums,
        })
    }
    if !frame.signals.is_empty() {
        entries.push(DocumentationEntry {
            entry_type: EntryType::SIGNAL,
            symbols: frame.signals,
        })
    }
    if !frame.exports.is_empty() {
        entries.push(DocumentationEntry {
            entry_type: EntryType::EXPORT,
            symbols: frame.exports,
        })
    }
    if !frame.constants.is_empty() {
        entries.push(DocumentationEntry {
            entry_type: EntryType::CONST,
            symbols: frame.constants,
        })
    }
    if !frame.functions.is_empty() {
        entries.push(DocumentationEntry {
            entry_type: EntryType::FUNC,
            symbols: frame.functions,
        })
    }
    if !frame.variables.is_empty() {
        entries.push(DocumentationEntry {
            entry_type: EntryType::VAR,
            symbols: frame.variables,
        })
    }
}

fn parse_class_content(
    filename: &str,
    lineno: u32,
    line: &str,
    indent: u32,
    frame: &mut ClassFrame,
    comment_buffer: &mut Vec<String>,
    settings: &Settings,
    override_visibility: &mut Option<bool>,
) -> Result<Option<Mode>, String> {
    if line.starts_with("class ") {
        let name = line[5..].split(':').next().unwrap().trim().to_string();

        if !name.starts_with("_") || settings.show_prefixed {
            return Ok(Some(Mode::Class(
                name,
                (indent, None),
                ClassFrame::default(),
                comment_buffer.drain(..).collect(),
            )));
        }
    } else if line.starts_with("signal ") {
        let name = line[6..].trim().to_string();
        if (!name.starts_with("_") || settings.show_prefixed) && override_visibility.unwrap_or(true)
        {
            frame.signals.push(Symbol {
                name: name,
                args: None,
                text: comment_buffer.drain(..).collect(),
            });
        }
    } else if line.starts_with("func ") {
        let mut name = String::new();
        let mut arguments = Vec::new();
        let mut super_arguments = None;
        let mut return_type = None;

        parse_function(
            &line[4..],
            &mut name,
            &mut arguments,
            &mut super_arguments,
            &mut return_type,
        )?;

        if (!name.starts_with("_") || settings.show_prefixed) && override_visibility.unwrap_or(true)
        {
            frame.functions.push(Symbol {
                name: name,
                args: Some(SymbolArgs::FunctionArgs(FunctionArgStruct {
                    arguments: arguments,
                    super_arguments: super_arguments,
                    return_type: return_type,
                })),
                text: comment_buffer.drain(..).collect(),
            });
        }
    } else if line.starts_with("var ") {
        let mut name = String::new();
        let mut value_type = None;
        let mut assignment = None;
        let mut setter = None;
        let mut getter = None;
        parse_assignment(
            filename,
            lineno,
            &line[4..],
            &mut name,
            &mut value_type,
            &mut assignment,
            &mut setter,
            &mut getter,
        )?;

        if (!name.starts_with("_") || settings.show_prefixed) && override_visibility.unwrap_or(true)
        {
            frame.variables.push(Symbol {
                name: name,
                args: Some(SymbolArgs::VariableArgs(VariableArgStruct {
                    value_type: value_type,
                    assignment: assignment,
                    setter: setter,
                    getter: getter,
                })),
                text: comment_buffer.drain(..).collect(),
            });
        }
    } else if line.starts_with("const ") {
        let mut name = String::new();
        let mut value_type = None;
        let mut assignment = None;
        let mut setter = None;
        let mut getter = None;
        parse_assignment(
            filename,
            lineno,
            &line[6..],
            &mut name,
            &mut value_type,
            &mut assignment,
            &mut setter,
            &mut getter,
        )?;

        if (!name.starts_with("_") || settings.show_prefixed) && override_visibility.unwrap_or(true)
        {
            frame.constants.push(Symbol {
                name: name,
                args: Some(SymbolArgs::VariableArgs(VariableArgStruct {
                    value_type: value_type,
                    assignment: assignment,
                    setter: setter,
                    getter: getter,
                })),
                text: comment_buffer.drain(..).collect(),
            });
        }
    } else if line.starts_with("export") {
        let pos = line.find(" var ");
        let open_paren = line.find('(');
        let close_paren = line.find(')');
        if pos.is_none() {
            return Err(format!("Invalid syntax: {}", line));
        }

        let pos = pos.unwrap();
        let export_type = match (open_paren, close_paren) {
            (Some(open), Some(close)) if open < close && close < pos => {
                let mut arg_iterator = line[open + 1..close]
                    .split(',')
                    .map(|x| x.trim().to_string());
                let export_type = arg_iterator.next();
                let options = arg_iterator.collect::<Vec<_>>();
                Some((export_type.unwrap(), options))
            }
            (Some(x), Some(y)) if x > pos && y > pos => None,
            (Some(x), None) if x > pos => None,
            (None, Some(x)) if x > pos => None,
            (None, None) => None,
            _ => return Err(format!("Invalid syntax: {}", line)),
        };

        let mut name = String::new();
        let mut value_type = None;
        let mut assignment = None;
        let mut setter = None;
        let mut getter = None;
        parse_assignment(
            filename,
            lineno,
            &line[pos + 5..],
            &mut name,
            &mut value_type,
            &mut assignment,
            &mut setter,
            &mut getter,
        )?;

        if (name.starts_with("_") && !settings.show_prefixed)
            || !override_visibility.unwrap_or(true)
        {
            return Ok(None);
        }

        let (export_type, options) = match export_type {
            Some((x, y)) => (Some(x), y),
            None => (None, Vec::new()),
        };

        frame.exports.push(Symbol {
            name: name,
            args: Some(SymbolArgs::ExportArgs(ExportArgStruct {
                value_type: export_type.or(value_type),
                options: options,
                assignment: assignment,
                setter: setter,
                getter: getter,
            })),
            text: comment_buffer.drain(..).collect(),
        });
    } else if line.starts_with("enum") {
        let pos = line.find('{');
        if pos.is_none() {
            return Err(format!("Invalid Syntax: {}", line));
        }

        let pos = pos.unwrap();
        let enum_name = line[5..pos].trim().to_string();

        if (enum_name.starts_with("_") && !settings.show_prefixed)
            || !override_visibility.unwrap_or(true)
        {
            return Ok(None);
        }

        let mut enum_frame = EnumFrame::default();
        let end = line.find('}');
        let slice = match end {
            Some(x) => &line[pos + 1..x],
            None => &line[pos + 1..],
        };
        for v in slice.split(',') {
            let mut arg_iterator = v.split('=');

            let name = arg_iterator.next().unwrap().trim().to_string();
            if name.trim().is_empty() {
                continue;
            }
            let value = arg_iterator
                .next()
                .and_then(|x| x.trim().parse().ok())
                .unwrap_or(enum_frame.last_value);

            enum_frame.last_value = value + 1;

            if (!name.starts_with("_") || settings.show_prefixed)
                && override_visibility.unwrap_or(true)
            {
                enum_frame.values.push(EnumValue {
                    name: name,
                    value: value,
                    text: Vec::new(),
                });
            }
        }

        if end.is_some() {
            frame.enums.push(Symbol {
                name: enum_name,
                args: Some(SymbolArgs::EnumArgs(enum_frame.values)),
                text: comment_buffer.drain(..).collect(),
            });
        } else {
            return Ok(Some(Mode::Enum(enum_name, enum_frame)));
        }
    }

    Ok(None)
}

enum MatchType {
    FAILURE,
    MATCH,
    FINISHED,
}

trait Predicate {
    fn into_matcher(self) -> Box<dyn Matcher>;
}

impl Predicate for char {
    fn into_matcher(self) -> Box<dyn Matcher> {
        Box::new(self)
    }
}

impl Predicate for &str {
    fn into_matcher(self) -> Box<dyn Matcher> {
        Box::new(StringMatcher {
            index: 0,
            chars: self.chars().collect(),
            len: self.len(),
        })
    }
}

trait Matcher {
    fn matches(&mut self, c: char) -> MatchType;
}

struct StringMatcher {
    index: usize,
    len: usize,
    chars: Vec<char>,
}

impl Matcher for char {
    fn matches(&mut self, c: char) -> MatchType {
        if c == *self {
            MatchType::FINISHED
        } else {
            MatchType::FAILURE
        }
    }
}

impl Matcher for StringMatcher {
    fn matches(&mut self, c: char) -> MatchType {
        if self.index > self.len || self.chars[self.index] != c {
            self.index = 0;
            MatchType::FAILURE
        } else if self.index == self.len - 1 && self.chars[self.index] == c {
            MatchType::FINISHED
        } else {
            self.index += 1;
            MatchType::MATCH
        }
    }
}

fn find(
    filename: &str,
    lineno: u32,
    s: &str,
    p: impl Predicate,
    previous_parentheses: Vec<char>,
) -> Result<(Option<usize>, Vec<char>), String> {
    let mut parentheses = previous_parentheses;
    let mut single_string = false;
    let mut double_string = false;

    let chars = s.chars().collect::<Vec<_>>();
    let len = chars.len();

    let mut matcher = p.into_matcher();
    for i in 0..len {
        if !single_string && !double_string {
            let mut j = 0;
            while i + j < len {
                let c = chars[i + j];
                j += 1;

                match matcher.as_mut().matches(c) {
                    MatchType::FAILURE => break,
                    MatchType::FINISHED => return Ok((Some(i), parentheses)),
                    _ => (),
                }
            }
        }

        match chars[i] {
            '"' if !single_string => double_string = true,
            '\'' if !double_string => single_string = true,
            x if x == '(' || x == '[' || x == '{' => parentheses.push(x),
            ')' => match parentheses.pop() {
                Some('(') => (),
                Some(_) => return Err(format!("Failed to parse {}, line {}: Closing parentheses does not match opening parentheses", filename, lineno)),
                None => return Err(format!("Failed to parse {}, line {}: extra ')'", filename, lineno))
            }
            ']' => match parentheses.pop() {
                Some('[') => (),
                Some(_) => return Err(format!("Failed to parse {}, line {}: Closing parentheses does not match opening parentheses", filename, lineno)),
                None => return Err(format!("Failed to parse {}, line {}: extra ']'", filename, lineno))
            }
            '}' => match parentheses.pop() {
                Some('{') => (),
                Some(_) => return Err(format!("Failed to parse {}, line {}: Closing parentheses does not match opening parentheses", filename, lineno)),
                None => return Err(format!("Failed to parse {}, line {}: extra '}}'", filename, lineno))
            }
            _ => (),
        }
    }

    Ok((None, parentheses))
}

fn parse_assignment(
    filename: &str,
    lineno: u32,
    line: &str,
    name: &mut String,
    value_type: &mut Option<String>,
    assignment: &mut Option<String>,
    setter: &mut Option<String>,
    getter: &mut Option<String>,
) -> Result<(), String> {
    let (assignment_pos, _) = find(filename, lineno, line, '=', Vec::new())?;
    let (type_pos, _) = find(filename, lineno, line, ':', Vec::new())?;
    let (setget_pos, _) = find(filename, lineno, line, " setget ", Vec::new())?;

    match (assignment_pos, type_pos, setget_pos) {
        (Some(apos), Some(tpos), Some(spos)) if tpos < apos && apos < spos => {
            let setget = &line[spos + 7..]
                .split(',')
                .map(|x| x.trim())
                .collect::<Vec<_>>();
            match setget.as_slice() {
                ["", get] => {
                    getter.get_or_insert(get.to_string());
                }
                [set] | [set, ""] => {
                    setter.get_or_insert(set.to_string());
                }
                [set, get] => {
                    setter.get_or_insert(set.to_string());
                    getter.get_or_insert(get.to_string());
                }
                _ => {
                    return Err(format!(
                        "Failed to parse {}, line {}: invalid syntax '{}'",
                        filename, lineno, line
                    ))
                }
            }
            name.clone_from(&line[..tpos].trim().to_string());
            value_type.get_or_insert(line[tpos + 1..apos].trim().to_string());
            assignment.get_or_insert(line[apos + 1..spos].trim().to_string());
        }
        (Some(apos), Some(tpos), None) if tpos < apos => {
            name.clone_from(&line[..tpos].trim().to_string());
            value_type.get_or_insert(line[tpos + 1..apos].trim().to_string());
            assignment.get_or_insert(line[apos + 1..].trim().to_string());
        }
        (Some(apos), None, Some(spos)) if apos < spos => {
            let setget = &line[spos + 7..]
                .split(',')
                .map(|x| x.trim())
                .collect::<Vec<_>>();
            match setget.as_slice() {
                ["", get] => {
                    getter.get_or_insert(get.to_string());
                }
                [set] | [set, ""] => {
                    setter.get_or_insert(set.to_string());
                }
                [set, get] => {
                    setter.get_or_insert(set.to_string());
                    getter.get_or_insert(get.to_string());
                }
                _ => {
                    return Err(format!(
                        "Failed to parse {}, line {}: invalid syntax '{}'",
                        filename, lineno, line
                    ))
                }
            }
            name.clone_from(&line[..apos].trim().to_string());
            assignment.get_or_insert(line[apos + 1..spos].trim().to_string());
        }
        (Some(apos), None, None) => {
            name.clone_from(&line[..apos].trim().to_string());
            assignment.get_or_insert(line[apos + 1..].trim().to_string());
        }
        (None, Some(tpos), Some(spos)) if tpos < spos => {
            let setget = &line[spos + 7..]
                .split(',')
                .map(|x| x.trim())
                .collect::<Vec<_>>();
            match setget.as_slice() {
                ["", get] => {
                    getter.get_or_insert(get.to_string());
                }
                [set] | [set, ""] => {
                    setter.get_or_insert(set.to_string());
                }
                [set, get] => {
                    setter.get_or_insert(set.to_string());
                    getter.get_or_insert(get.to_string());
                }
                _ => {
                    return Err(format!(
                        "Failed to parse {}, line {}: invalid syntax '{}'",
                        filename, lineno, line
                    ))
                }
            }
            name.clone_from(&line[..tpos].trim().to_string());
            value_type.get_or_insert(line[tpos + 1..spos].trim().to_string());
        }
        (None, Some(tpos), None) => {
            name.clone_from(&line[..tpos].trim().to_string());
            value_type.get_or_insert(line[tpos + 1..].trim().to_string());
        }
        (None, None, Some(spos)) => {
            let setget = &line[spos + 7..]
                .split(',')
                .map(|x| x.trim())
                .collect::<Vec<_>>();
            match setget.as_slice() {
                ["", get] => {
                    getter.get_or_insert(get.to_string());
                }
                [set] | [set, ""] => {
                    setter.get_or_insert(set.to_string());
                }
                [set, get] => {
                    setter.get_or_insert(set.to_string());
                    getter.get_or_insert(get.to_string());
                }
                _ => {
                    return Err(format!(
                        "Failed to parse {}, line {}: invalid syntax '{}'",
                        filename, lineno, line
                    ))
                }
            }
            name.clone_from(&line[..spos].trim().to_string());
        }
        (None, None, None) => {
            name.clone_from(&line.trim().to_string());
        }
        _ => {
            return Err(format!(
                "Failed to parse {}, line {}: invalid syntax '{}'",
                filename, lineno, line
            ))
        }
    };

    Ok(())
}

fn parse_function(
    line: &str,
    name: &mut String,
    arguments: &mut Vec<FunctionArgument>,
    super_arguments: &mut Option<Vec<FunctionArgument>>,
    return_type: &mut Option<String>,
) -> Result<(), String> {
    #[derive(PartialEq)]
    enum SIDE {
        Name,
        Type,
        Assignment,
        Invalid,
    }

    let mut finished = false;

    let mut depth = 0;
    let mut parentheses_count = 0;
    let mut side = SIDE::Name;
    let mut last_char = None;

    let mut current_argument_name = String::new();
    let mut current_argument_type = None;
    let mut current_argument_assignment = None;
    for c in line.chars() {
        match c {
            x if x.is_whitespace() => (),
            _ if finished => return Err(format!("Invalid syntax: {}", line)),
            '(' => {
                if parentheses_count < 2 {
                    depth += 1
                } else {
                    return Err(format!("Invalid syntax: {}", line));
                }
            }
            ')' => {
                depth -= 1;
                if depth == 0 && !current_argument_name.is_empty() {
                    match parentheses_count {
                        0 => {
                            arguments.push(FunctionArgument {
                                name: current_argument_name,
                                value_type: current_argument_type,
                                default_value: current_argument_assignment,
                            });
                            current_argument_name = String::new();
                            current_argument_type = None;
                            current_argument_assignment = None;
                        }
                        1 => {
                            super_arguments
                                .get_or_insert(Vec::new())
                                .push(FunctionArgument {
                                    name: current_argument_name,
                                    value_type: current_argument_type,
                                    default_value: current_argument_assignment,
                                });
                            current_argument_name = String::new();
                            current_argument_type = None;
                            current_argument_assignment = None;
                        }
                        _ => return Err(format!("Invalid syntax: {}", line)),
                    }
                }
                if depth == 0 {
                    side = SIDE::Invalid;
                    parentheses_count += 1;
                }
            }
            '.' if depth == 0 && name == "_init" && parentheses_count == 1 => side = SIDE::Name,
            '.' if depth == 0 => return Err(format!("Invalid syntax: {}", line)),
            ':' if depth == 0 => finished = true,
            ':' => {
                side = SIDE::Type;
                current_argument_type = Some(String::new());
            }
            ',' => {
                match parentheses_count {
                    0 => {
                        arguments.push(FunctionArgument {
                            name: current_argument_name,
                            value_type: current_argument_type,
                            default_value: current_argument_assignment,
                        });
                        current_argument_name = String::new();
                        current_argument_type = None;
                        current_argument_assignment = None;
                    }
                    1 => {
                        super_arguments
                            .get_or_insert(Vec::new())
                            .push(FunctionArgument {
                                name: current_argument_name,
                                value_type: current_argument_type,
                                default_value: current_argument_assignment,
                            });
                        current_argument_name = String::new();
                        current_argument_type = None;
                        current_argument_assignment = None;
                    }
                    _ => return Err(format!("Invalid syntax: {}", line)),
                };
            }
            '-' if depth == 0 => (),
            '>' => {
                if last_char == Some('-') {
                    side = SIDE::Type;
                } else {
                    return Err(format!("Invalid syntax: {}", line));
                }
            }
            '=' if depth == 1 && side != SIDE::Assignment => side = SIDE::Assignment,
            x if depth == 0 && side == SIDE::Name => name.push(x),
            x if depth == 0 && side == SIDE::Type => {
                return_type.get_or_insert(String::new()).push(x)
            }
            x if side == SIDE::Name => current_argument_name.push(x),
            x if side == SIDE::Type => {
                current_argument_type = current_argument_type.map(|mut s| {
                    s.push(x);
                    s
                })
            }
            x if side == SIDE::Assignment => current_argument_assignment
                .get_or_insert(String::new())
                .push(x),
            _ if side == SIDE::Invalid => return Err(format!("Invalid syntax: {}", line)),
            _ => panic!("parse_function: Some case not covered"),
        };
        last_char = Some(c);
    }

    Ok(())
}
