use std::{fmt::{Display, Debug}, collections::HashMap, hash::Hash};

#[derive(Clone, PartialEq)]
pub enum Value {
    String(String), Char(char), Int(i64), Float(f64), Boolean(bool)
}
impl Value {
    pub fn typ(&self) -> Type {
        match self {
            Self::String(_) => Type::String,
            Self::Char(_) => Type::Char,
            Self::Int(_) => Type::Int,
            Self::Float(_) => Type::Float,
            Self::Boolean(_) => Type::Boolean,
        }
    }
}
impl Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::String(string) => write!(f, "{string:?}"),
            Self::Char(char) => write!(f, "{char:?}"),
            Self::Int(int) => write!(f, "{int:?}"),
            Self::Float(float) => write!(f, "{float:?}"),
            Self::Boolean(boolean) => write!(f, "{boolean:?}"),
        }
    }
}
impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::String(string) => write!(f, "{string}"),
            Self::Char(char) => write!(f, "{char}"),
            Self::Int(int) => write!(f, "{int}"),
            Self::Float(float) => write!(f, "{float}"),
            Self::Boolean(boolean) => write!(f, "{boolean}"),
        }
    }
}
#[derive(Clone, Copy, Eq)]
pub enum Type {
    Any,
    String, Char, Int, Float, Boolean
}
impl PartialEq for Type {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Any, _) | (_, Self::Any) => true,
            (Self::String, Self::String) => true,
            (Self::Char, Self::Char) => true,
            (Self::Int, Self::Int) => true,
            (Self::Float, Self::Float) => true,
            (Self::Boolean, Self::Boolean) => true,
            _ => false
        }
    }
}
impl Debug for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Any => write!(f, "any"),
            Self::String => write!(f, "str"),
            Self::Char => write!(f, "char"),
            Self::Int => write!(f, "int"),
            Self::Float => write!(f, "float"),
            Self::Boolean => write!(f, "bool"),
        }
    }
}
impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl Hash for Type {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        
    }
    fn hash_slice<H: std::hash::Hasher>(data: &[Self], state: &mut H)
        where
            Self: Sized, {
        
    }
}
