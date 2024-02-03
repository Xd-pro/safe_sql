use std::collections::{HashMap, VecDeque};

#[derive(Debug, PartialEq, Clone)]
pub enum Thing {
    Comment(String),
    Sql(String),
    EndOfQuery()
}

#[derive(Debug)]
pub struct SyntaxError {

}

pub fn lex(text: String) -> Result<VecDeque<Thing>, SyntaxError> {
    let mut double_quotes = false;
    let mut single_quotes = false;
    let mut backticks = false;

    let mut in_comment = false;

    let mut is_next_dash = false;

    let mut things: Vec<Thing> = Vec::new();

    let mut current = String::new();
    
    for (i, char) in text.chars().enumerate() {
        if in_comment {
            if char == '\n' {
                in_comment = false;
                things.push(Thing::Comment(current));
                current = String::new();
            } else {
                if !char.is_ascii_control() && !is_next_dash {
                    current.push(char); 
                }
                is_next_dash = false;
            }
        } else {
            if !double_quotes && !single_quotes && !backticks {
                match text.chars().nth(i + 1) {
                    None => {}
                    Some(next_char) => {
                        if char == '-' && next_char == '-' {
                            if current != "" {
                                things.push(Thing::Sql(current));
                            }
                            current = String::new();
                            in_comment = true;
                            is_next_dash = true;
                        } else {
                            
                        }
                    }
                }
            }          
        }
        if !in_comment {
            match text.chars().nth(i - 1) {
                None => {}
                Some(last_char) => {
                    if last_char != '\\' {
                        if char == '"' {
                            double_quotes = !double_quotes;
                        } else if char == '\'' {
                            single_quotes = !single_quotes;
                        } else if char == '`' {
                            backticks = !backticks;
                        }
                    }
                }
            }
            let mut push = true;
            if !double_quotes && !single_quotes && !backticks {
                if char == ';' {
                    push = false;
                    things.push(Thing::Sql(current));
                    things.push(Thing::EndOfQuery());
                    current = String::new();
                } else if char.is_ascii_control() {
                    push = false;
                }
            }
            if push {
                current.push(char);
            }
        }
    }

    let mut without_comments: Vec<&Thing> = Vec::new();

    for thing in &things {
        match thing {
            Thing::Comment(_) => {}
            t => {
                without_comments.push(t);
            }
        }
    }

    match without_comments.last() {
        Some(last) => {
            match **last {
                Thing::EndOfQuery() => {}
                _ => {
                    return Err(SyntaxError {  });
                }
            }
        }
        _ => {}
    }

    return Ok(VecDeque::from(things));

}

#[derive(Debug)]
pub struct FormatError;

pub fn lex_2(filename: String, mut input: VecDeque<Thing>, mut base: HashMap<String, String>) -> Result<HashMap<String, String>, FormatError> {

    let mut data: VecDeque<Thing> = VecDeque::new();
    while (|| {
        match input.pop_front() {
            Some(thing) => {
                data.push_back(thing.clone());
                thing != Thing::EndOfQuery()
            }
            None => {
                false
            }
        }
    })() {
        
    }

    let mut name: String = String::new();

    match data.pop_front() {
        Some(thing) => {
            match thing {
                Thing::Comment(text) => {
                    if text.starts_with("#") {
                        name = text.get(1..).unwrap().to_string()
                    } else {
                        return Err(FormatError)
                    }
                }
                _ => {
                    println!("155{:?}", data);
                    return Err(FormatError)
                }
            }
        }
        None => {
            
        }
    }

    if name == "" {
        return Ok(base)
    }

    let _ = base.insert(filename.clone() + "_" + &name, "".to_string());

    for thing in data {
        match thing {
            Thing::Sql(text) => {
                base.get_mut(&(filename.clone() + "_" + &name)).unwrap().push_str(&text);
            }
            _ => {}
        }
    }

    

    if !input.back().is_none() {
        match lex_2(filename, input, base) {
            Err(err) => {
                return Err(err);
            }
            Ok(rec_output) => {
                base = rec_output;
            }
        }
    }

    Ok(base)
    
}

#[derive(Debug, Clone)]

pub enum SqlToken {
    Sql(String),
    Return(String, String), // name, php type name
    Variable(String, String)
}

pub fn lex_sql(mut sql: String) -> Vec<SqlToken> {

    sql = sql + " ";

    let mut double_quotes = false;
    let mut single_quotes = false;
    let mut backticks = false;

    let mut in_variable = false;
    let mut in_return = false;
    let mut found_char = false;

    let mut out: Vec<SqlToken> = Vec::new();

    let mut current = String::new();
    let mut type_name = String::new();

    let mut past_colon = false;

    for (i, char) in sql.chars().enumerate() {
        if in_variable || in_return {
            if !past_colon {
                if char == ':' {
                    past_colon = true;
                    continue;
                }
                current.push(char);
            } else {
                if found_char {
                    if !char.is_whitespace() && (char.is_alphanumeric() || char == '_' || char == '?') {
                        type_name.push(char);
                    } else {
                        if in_return {
                            out.push(SqlToken::Return(current.clone(), type_name.clone()));
                        } else {
                            out.push(SqlToken::Variable(current.clone(), type_name.clone()));
                        }
                        current = String::new();
                        current.push(char);
                        type_name = String::new();
                        in_return = false;
                        in_variable = false;
                        past_colon = false;
                    }
                } else {
                    if !char.is_whitespace() && (char.is_alphanumeric() || char == '_' || char == '?') {
                        type_name.push(char);
                        found_char = true;
                    }
                }
            }
        } else {
            if !single_quotes && !double_quotes && !backticks {
                if char == '"' || char == '\'' || char == '`' {
                    match sql.chars().nth(i - 1) {
                        Some(prev_char) => {
                            if prev_char != '\\' {
                                if char == '\'' {
                                    single_quotes = !single_quotes;
                                }
                                if char == '"' {
                                    double_quotes = !double_quotes;
                                }
                                if char == '`' {
                                    backticks = !backticks;
                                }
                            }
                        }
                        None => {
                            
                        }
                    }
                }
            }
            if char == '$' {
                in_variable = true;
                out.push(SqlToken::Sql(current.clone()));
                current = String::new();
                past_colon = false;
                type_name = String::new();
                found_char = false;
            } else if char == '@' {
                in_return = true;
                out.push(SqlToken::Sql(current.clone()));
                current = String::new();
                past_colon = false;
                type_name = String::new();
                found_char = false;
            } else {
                current.push(char);
            }
        }
    }

    if !in_return && !in_variable {
        out.push(SqlToken::Sql(current));
    }

    out

}