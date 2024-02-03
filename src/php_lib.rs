use string_builder::Builder;

pub struct Class {

    pub name: String,
    pub extends: Option<String>,
    pub implements: Vec<String>,
    pub is_abstract: bool,
    pub members: Vec<Box<dyn ClassMember>>,
    pub comment: String

}

pub trait ClassMember: ToString {}

#[derive(Debug, Clone)]
pub enum Visibility {
    Public(),
    Private(),
    Protected()
}

impl ToString for Visibility {
    fn to_string(&self) -> String {
        match self {
            Visibility::Public() => "public ".to_string(),
            Visibility::Private() => "private ".to_string(),
            Visibility::Protected() => "protected ".to_string(),
        }
    }
}

impl ToString for Class {
    fn to_string(&self) -> String {
        let mut b = Builder::default();
        if self.is_abstract {
            b.append("abstract ");
        }
        b.append("class ");
        b.append(self.name.as_str());
        b.append(" ");
        match &self.extends {
            Some(extends) => {
                b.append("extends ");
                b.append(extends.as_str());
                b.append(" ");
            }
            None => {}
        }
        if !self.implements.is_empty() {
            b.append("implements ");
            let mut len = 0;
            for _ in &self.implements {
                len+=1;
            }
            println!("{}", len);
            for implement in &self.implements {
                len -= 1;
                b.append(implement.as_str());
                if len >= 1 {
                    b.append(", ");
                } else {
                    b.append(" ");
                }
            }
        }
        b.append("{");
        for member in &self.members {
            b.append(member.to_string());
        }
        b.append("}");
        b.string().unwrap()
    }
}

pub struct Function {
    
    pub name: String,
    pub params: Vec<Param>,
    pub body: Vec<Box<dyn ToString>>,
    pub visibility: Option<Visibility>,
    pub comment: String

}

impl ToString for Function {
    fn to_string(&self) -> String {
        let mut b = Builder::default();

        b.append(self.comment.as_str());
        
        match &self.visibility {
            Some(visibility) => {b.append(visibility.to_string())}
            None => {}
        }

        b.append("function ");

        b.append(self.name.as_str());

        b.append("(");

        for param in &self.params {
            b.append(match &param.visibility {
                Some(visibility) => visibility.to_string(),
                None => "".to_string()
            });
            b.append(param.param_type.as_str());
            b.append(" $");
            b.append(param.name.as_str());
            b.append(",");
        }

        b.append(") {");

        for thing in &self.body {
            b.append(thing.to_string());
        }

        b.append("}");


        b.string().unwrap()
    }
}

impl ClassMember for Function {
    
}

#[derive(Debug, Clone)]
pub struct Param {
    
    pub name: String,
    pub param_type: String,
    pub visibility: Option<Visibility>

}