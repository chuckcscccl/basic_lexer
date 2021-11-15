//! basic_lexer is a basic lexical scanner designed for use as the
//! first stage of compiler construction, and produces tokens required by
//! a parser.  It was originally intended to support the parallel project *RustLr*,
//! which is a LR-style parser generator, although each project is independent
//! of the other. It is not intended to be the best possible lexical scanner as it
//! does not build a DFA from regular expressions.  It does not count white-spaces
//! so it would not be appropriate for scanning python-like syntax.  However, as
//! this author did not find a satisfactory solution that suited all his needs,
//! he was compelled to create his own.  It is a component of a compiler that can
//! be improved upon modularly.
//!
//! The structures [Str_tokenizer] and [File_tokenizer] both implement
//! [Iterator] and return [Token]s.  File_tokenizer is more full-featured and
//! recognizes multi-line string literals and multi-line comments, with
//! the option of keeping the comments as special tokens. The File_tokenizer
//! is also capable of distinguishing keywords (such as "if", "else", "while")
//! from other alphanumeric tokens.
//!
//! Example:
//!```ignore
//!  let mut fscan = File_tokenizer::new("test.c");
//!  fscan.add_keywords("while return");
//!  fscan.set_line_comment("//");
//!  fscan.set_keep_comments(false);
//!  fscan.set_keep_newline(true);
//!  while let Some(token) = fscan.next() {
//!    println!("{:?}, line {}",&token,fscan.line_number());
//!  }
//!```
//! The supplied *main.rs* and file *test.c* on github contain additional usage examples.



#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(unused_parens)]
#![allow(unused_mut)]
#![allow(unused_imports)]
#![allow(unused_assignments)]
// #[doc(inline)]

use std::io::{Read,Error,BufRead,BufReader,Lines,Result};
use std::cell::{RefCell,Ref,RefMut};
use std::rc::Rc;
use std::fs::File;
use std::collections::{HashMap,HashSet};
use crate::Token::*;


/// Tokens are returned by the iterators [Str_tokenizer] and [File_tokenizer].
#[derive(Clone,PartialEq,Debug)]
pub enum Token
{
   /// non-negative base-10 integers.  It is however, more convenient to
   /// capture the value as a signed integer despite the fact that
   /// negative signs are emitted as separate symbols.
   Integer(i64), /// non-negative base-10 floating-point numbers
   Float(f64),
   /// non-alphanumeric symbols such as "==".  Note that a substring such as
   /// "=*=" will also be recognized as a single Symbol token, unless one of 
   /// these characters is designed as a 'singleton' by the
   /// [File_tokenizer::add_singletons] function.
   Symbol(String),
   /// tokens that must start with a alphabetical character or '\_',
   /// followed an arbitrary number of alphanumeric or '\_' symbols.
   Alphanum(String),
   /// special keywords such as "if", "else", "while" that are distinguished
   /// from other alphanumeric tokens.  Keywords can be added to
   /// File_tokenizer with the [File_tokenizer::add_keywords] function.
   Keyword(String),
   /// string literals, which can span multiple lines if produced by
   /// File_tokenizer.  String literals always include the enclosing
   /// \" characters.
   Stringlit(String),
   /// Verbatim, non-tokenized text such as comments, only produced by
   /// File_tokenizer with the keep_comments option
   Verbatim(String),
   /// indicates that a new line has been read; only produced by
   /// File_tokenizer with the keep_newline option
   Newline,
   #[doc(hidden)]
   Nothing,
   #[doc(hidden)]   
   Error(&'static str),
}

fn isdelim(c:char) -> bool
{
  c=='(' || c=='[' || c=='{' || c==')' || c==']' || c=='}'
}

// returns token and next index
fn next_token(s:&str) -> (Token, usize) 
{
  //let st = s.trim_start();   // assume s already trimmed
  if s.len()<1 {return (Newline,0);}
  let first = s.chars().next().unwrap();
  let mut index = 0;
  if first.is_alphabetic() || first=='_' {
     index=match_alphanum(s);
     return (Alphanum(String::from(&s[0..index])),index);
  }//alphanumeric
  else if first=='.' || first.is_ascii_digit() { //possible number
     index = match_num(s);
     if index>0 {
        if let Ok(m) = s[0..index].parse::<i64>() {return (Integer(m),index);}
        else if let Ok(n) = s[0..index].parse::<f64>()
          {return (Float(n),index);}
     }
     else {return (Symbol(first.to_string()),1);}
  }
  else if first=='\"' {
     index = match_strlit(s);
     if index>0
       {return (Stringlit(String::from(&s[0..index])),index);}  //includes ""
     else {return (Verbatim(s.to_string()),s.len());}
  }
  else {
     index = match_symbol(s);
     if index>0 {return (Symbol(String::from(&s[0..index])),index);}
  }
  (Verbatim(s.to_string()),s.len())  // default
}

///// match token type, returns index of where token ended plus one.
fn match_alphanum(s:&str) -> usize
{
   if s.len()<1 {return 0;}
   let mut chars = s.chars();
   let mut next = chars.next().unwrap();
   if !(next.is_alphabetic() || next=='_') {return 0;}
   let mut index = 1;
   let mut stop=false;
   while !stop && index<s.len()
   {
      next = chars.next().unwrap();
      if next.is_alphanumeric() || next=='_' { index+=1; }
      else {stop=true;}
   }
   return index;
}//match_alphanum

fn match_num(s:&str) -> usize  // could be integer or float
{
   let mut chars = s.chars();
   let mut index = 0;
   let mut stop = false;
   let mut foundpoint = false;
   let mut founddigit = false;
   while !stop && index<s.len()
   {
     let mut next = chars.next().unwrap();
     match (next, foundpoint) {
       ('.', false) => { foundpoint=true; index+=1},
       ('.', true) => {stop=true; index=0}, // match failed (2 .'s)
       (x,_) if x.is_ascii_digit() => { founddigit=true; index+=1},
       _ => {stop=true;}
     }//match
   }
   if founddigit {index} else {0}
}

fn match_strlit(s:&str) -> usize
{
   if s.len()<1 {return 0;}
   let mut chars = s.chars();
   let mut index = 0;
   let mut stop = false;
   let mut next = chars.next().unwrap();
   if next !='\"' {return 0;} else { index+= 1; } // will only be called if...
   while !stop && index<s.len()
   {
      next = chars.next().unwrap();
      index += 1;
      if next=='\\' && index<s.len() {
         next=chars.next().unwrap();
         index += 1;
      } //skip escape char
      else if next=='\"' { stop=true; }
      //else {index+=1; }
   }//while
   if stop {index} else {0}
}  // the stringlit will include the enclosing ""'s

fn match_symbol(s:&str) -> usize
{
   if s.len()<1 {return 0;}
   let mut chars = s.chars();
   let mut c = chars.next().unwrap();
   if isdelim(c) {return 1;}
   if c.is_ascii_digit() || c.is_alphanumeric() || c=='_' || c=='\"' || c=='.' {return 0;}
   let mut index = 1;
   let mut stop = false;
   while !stop && index<s.len()
   {
     c = chars.next().unwrap();
     if !c.is_whitespace() && !c.is_ascii_digit() && !c.is_alphanumeric() && c!='_' && c!='\"' && c!='.' && !isdelim(c)
     {index+=1;} else {stop=true;}
   }
   return index;
}


///////////////////////////////////
/// A [Token] [Iterator] on a given &str.
///
/// Example
/// ```ignore
/// let mut scanner = Str_tokenizer::new("while (1) fork();");
/// while let Some(token) = scanner.next() {
///   println!("{:?}",&token);
/// }
/// /* this produces output:
///   Alphanum("while")
///   Symbol("(")
///   Integer(1)
///   Symbol(")")
///   Alphanum("fork")
///   Symbol("(")
///   Symbol(")")
///   Symbol(";")
/// */
/// ```
pub struct Str_tokenizer<'t>
{
  slice : &'t str,
  line_comment : char,
}
impl<'t> Str_tokenizer<'t>
{
   pub fn new(s:&'t str) -> Str_tokenizer<'t>
   {
      Str_tokenizer {
         slice : s.trim_start(),
         line_comment: '#', // use a whitespace for no linecomment
      }
   }
   /// returns the untokenized remainder of the str
   pub fn rest(&self) -> &'t str
   {     self.slice     }
   /// sets the character to designed a comment.  The rest of the input
   /// is skipped after this character.  The default line-comment character
   /// is #.
   pub fn set_line_comment(&mut self, c:char) {self.line_comment='c';}
   /// disables recognition of the line-comment character
   pub fn no_line_comment(&mut self) {self.line_comment=' ';}
   /// resets the tokenizer to recognize a new str.  The next [Token]
   /// emitted by [Iterator::next()] will be from the new string.
   pub fn reset<'u:'t>(&mut self, s:&'u str)
   {
      self.slice = s;
   }
}//impl Str_tokenizer

impl<'t> Iterator for Str_tokenizer<'t>
{
  type Item = Token;
  fn next(&mut self) -> Option<Token>
  {
     if self.slice.len()<1 {None}
     else {
        let (tok,ind) = next_token(self.slice);
        let mut return_value = None;
         match tok {
           Symbol(s) if s.chars().next().unwrap()==self.line_comment => {
             return_value = Some(Verbatim(self.slice.to_owned()));
             self.slice = "";
           },
           /*
           Alphanum(a) => {
             if let Some(hsref) = self.keywords {
               if hsref.contains(&a) {return_value = Some(Keyword(a));}
             }
           },
           */
           _ =>  { return_value = Some(tok); },
         };
        if self.slice.len()>0 {self.slice = &self.slice[ind..].trim_start();}
        return_value
     }
  }//next
}//impl Iterator for Str_tokenizer

/*
//////////////////////////////////////////////////////////////////
//////////////// File_tokenizer //////////////////////////////////
//////////////////////////////////////////////////////////////////
*/

#[derive(PartialEq,Eq,Copy,Clone,Debug)]
enum Mode {
  normal,
  comment(usize),   // usize keeps starting linenum of comment 
  stringlit(usize), // for error reporting
}
fn iscomment(m:&Mode) -> bool
{ if let Mode::comment(n) = m {true} else {false} }
fn isstringlit(m:&Mode) -> bool
{ if let Mode::stringlit(n) = m {true} else {false} }

///////////////////////////
/// a [Token] [Iterator] on a given file
pub struct File_tokenizer
{ 
  linenum: usize, 
  column: usize,  
  line : Rc<RefCell<String>>,
  reader : BufReader<File>,
  mode : Mode,  // 
  current_string : String,
  keywords: HashSet<String>,
  singletons: HashSet<char>, // singleton symbols
  line_comment : String,
  keep_comments : bool,
  begin_comment : String,
  end_comment : String,
  keep_newline : bool,
}//File_tokenizer
impl File_tokenizer
{
  /// creates a File_tokenizer given a file path, panics if file is not found
  pub fn new(filename:&str) -> File_tokenizer
  {
    let reader1 = match File::open(filename) {
          Ok(fi) => BufReader::new(fi),
          _ => {panic!("File {} not found",filename)},
        };
    let mut singlesyms = HashSet::<char>::new();
    let mut kwhash = HashSet::<String>::new();
    for c in "[](){}".chars() {singlesyms.insert(c);}
    File_tokenizer {
      linenum: 0,
      column: 0,
      line: Rc::new(RefCell::new(String::from(""))),
      reader : reader1,
      keywords: kwhash,
      singletons: singlesyms,
      line_comment: String::from("//"),
      keep_comments : false,
      keep_newline : false,
      mode : Mode::normal,
      current_string : String::from(""),
      begin_comment : String::from("/*"),
      end_comment : String::from("*/"),
    }
  }//new

   /// returns the current line number being read
   pub fn line_number(&self)->usize {self.linenum}
   /// returns the current column (character position) on the current line
   pub fn column_number(&self)->usize {self.column}
   /// returns a copy of the current line being tokenized
   pub fn current_line<'t>(&'t self)-> String
   {
        self.line.borrow().clone()
   }

   /// adds keywords that are to be distinguished from other alphanumeric
   /// tokens.  Keywords are returned as [Keyword] tokens.  Keywords
   /// are specified using a whitespace-separated string.
   ///
   /// Example:
   /// ```ingore
   /// let mut scanner = File_tokenizer::new("./somefilepath");
   /// scanner.add_keywords("if else while for return break");
   /// ```
   pub fn add_keywords(&mut self, kws:&str)
   {
      let ki = kws.split_whitespace();
      for kw in ki { self.keywords.insert(kw.trim().to_owned());}
   }
   /// adds characters to be recognized as single-character [Symbol] tokens.
   /// For example, if '=' is added as a singleton then "==" will be scanned
   /// as two separate [Symbol]s.  The default singletons are the brackets
   /// ( ) { } \[ and \].  These characters are always recognized as singleton
   /// symbols.
   ///
   /// Example
   /// ```ingore
   /// let mut scanner = File_tokenizer::new("./somefilepath");
   /// scanner.add_singletons(".*");
   /// ```
   /// Each character in the given string will be added to the default singletons.
   /// White-space and alphanumeric characters are ignored.
   pub fn add_singletons(&mut self, singles:&str)
   {
      for c in singles.chars() {self.singletons.insert(c);}
   }
   /// sets the symbol used to designate a single-line comment.  The
   /// default symbol is "//".  The rest of the line is skipped after
   /// this symbol.
   pub fn set_line_comment(&mut self, c:&str)
   {if c.len()>0 {self.line_comment=String::from(c.trim());} }
   /// sets the symbols used to delineate possibly multiple-line comments.
   /// The default comment delimiters are "/\*" and "\*/".  The argument *s* should
   /// be a whitespace-separated string (e.g. "\/* */").  The function has
   /// no effect if the argument is not of the right form.
   pub fn set_comments(&mut self, s:&str)
   {
      let cs:Vec<_> = s.split_whitespace().collect();
      if cs.len()!=2 {return;}
      self.begin_comment = cs[0].to_owned();
      self.end_comment = cs[1].to_owned();
   }
   /// disables the recogniton of single-line comments.  This option
   /// can only be re-enabled with [Self::set_line_comment].
   pub fn no_line_comment(&mut self) {self.line_comment=String::from("");}
   /// sets option to keep comments delineated by set_comments as [Verbatim] tokens. Default is false (does not keep comments)
   pub fn set_keep_comments(&mut self, b:bool) {self.keep_comments=b;}
   /// sets option to emit the [Newline] token when a new line is read
   /// other than the first line.  Newline is never emitted if inside
   /// a string literal or multi-line comment.  The default is false.
   pub fn set_keep_newline(&mut self, b:bool) {self.keep_newline=b;}   

   // move some match procedures here     INSIDE File_tokenizer ***
 // returns token and next index
 fn next_token(&mut self, s:&str) -> (Token, usize) 
 {
  //let st = s.trim_start();   // assume s already trimmed
  if s.len()<1 {return (Newline,0);}
  let first = s.chars().next().unwrap();
  let mut index = 0;
  if (s.len()>1 && &s[0..2]==&self.begin_comment[..] && self.mode==Mode::normal) || iscomment(&self.mode) {
     if !iscomment(&self.mode) {self.mode = Mode::comment(self.linenum);}
     match s.find(&self.end_comment) {
       Some(index) => {
          self.mode=Mode::normal;
          let mut ret=std::mem::replace(&mut self.current_string,String::from(""));
          if self.keep_comments {
             ret.push_str(&s[0..index+self.end_comment.len()]);
             return (Verbatim(ret), index+self.end_comment.len());
          }
          else { return (Nothing,index+self.end_comment.len()); }
       },
       None => {
          self.current_string.push_str(s);
          return (Nothing,s.len());
       },
     }//match
  }//comment mode
  else
  if first=='\"' || isstringlit(&self.mode) {
     if !isstringlit(&self.mode) {self.mode = Mode::stringlit(self.linenum);}
     index = self.match_strlit(s);
     if index>0  { // found closing quote
        let mut ret=std::mem::replace(&mut self.current_string,String::from(""));
        ret.push_str(&s[0..index]);
        self.mode = Mode::normal;
        return (Stringlit(ret),index);
       }  
     else {
        self.current_string.push_str(s);
        return (Nothing,s.len());
     }     
  }
  else if first.is_alphabetic() || first=='_' {
     index=match_alphanum(s);
     return (Alphanum(String::from(&s[0..index])),index);
  }//alphanumeric
  else if first=='.' || first.is_ascii_digit() { //possible number
     index = match_num(s);
     if index>0 {
        if let Ok(m) = s[0..index].parse::<i64>() {return (Integer(m),index);}
        else if let Ok(n) = s[0..index].parse::<f64>()
          {return (Float(n),index);}
     }
     else {return (Symbol(first.to_string()),1);}
  }
  else {
     index = self.match_symbol(s);
     if index>0 {return (Symbol(String::from(&s[0..index])),index);}
  }
  //(Verbatim(s.to_string()),s.len())  // default
  (Nothing,s.len())
 }//next_token inside impl File_tokenizer

 fn match_strlit(&mut self, s:&str) -> usize
 {
   if s.len()<1 {return 0;}
   let mut chars = s.chars();
   let mut index = 0;
   let mut stop = false;
   let mut next = chars.next().unwrap();
   /*if next !='\"' && self.current_string.len()==0 {return 0;}
   else*/ if next=='\"' && self.current_string.len()>0 { return 1; }
   // above else-if is for special empty string ""
   index +=1;
   while !stop && index<s.len()
   {
      next = chars.next().unwrap();
      index += 1;
      if next=='\\' && index<s.len() {
         next=chars.next().unwrap();
         index += 1;
      } //skip escape char
      else if next=='\"' { stop=true; }
   }//while
   if stop {index} else {0}
 } // the stringlit will include the enclosing ""'s - inside File_tokenizer

fn match_symbol(&mut self, s:&str) -> usize
{
   if s.len()<1 {return 0;}
   let mut chars = s.chars();
   let mut c = chars.next().unwrap();
   //if isdelim(c) {return 1;}
   if self.singletons.contains(&c) {return 1;} // special case
   if c.is_ascii_digit() || c.is_alphanumeric() || c=='_' || c=='\"' || c=='.' {return 0;}
   let mut index = 1;
   let mut stop = false;
   while !stop && index<s.len()
   {
     c = chars.next().unwrap();
     if !c.is_whitespace() && !c.is_ascii_digit() && !c.is_alphanumeric() && c!='_' && c!='\"' && c!='.' && !self.singletons.contains(&c)
     {index+=1;} else {stop=true;}
   }
   return index;
}

}//impl File_tokenizer

//////////
impl Iterator for File_tokenizer
{
  type Item = Token;
  fn next(&mut self) -> Option<Token>
  {
     let cmlen = self.line_comment.len();
     let mut return_value = None;     
     loop   // simulate do-while loop
     {  
       // goto next line if needed:
       let linerc = Rc::clone(&self.line);
       let mut brline = linerc.borrow_mut();
       if self.column >= brline.len() {
          let mut newline = String::from("");
          match self.reader.read_line(&mut newline) {
            Ok(n) if n>0 => {
               //self.line.replace(newline);
               *brline = newline;
               self.linenum +=1;
               self.column=0;
            },
            _ => {
              match self.mode {
                Mode::stringlit(n) => {
                   panic!("UNCLOSED STRING LITERAL STARTING ON LINE {}",n);
                },
                Mode::comment(n) => {
                   panic!("UNCLOSED COMMENT STARTING ON LINE {}",n);
                },
                _ => {return None;},
              }//match self.mode
            },
          }//match nextline
       } // read new line
       let slice0 = &brline[self.column..];
       let mut slice = slice0.trim_start();
       let diff = slice0.len() - slice.len();
       self.column += diff;
       let (tok,ind) = self.next_token(slice);
       let lcm = &self.line_comment[..]; // [..] makes slice borrow
       match tok {
           Symbol(s) if cmlen>0 && s.len()>=cmlen && &s[0..cmlen]==lcm => {
             return_value = if self.keep_comments {Some(Verbatim(String::from(slice)))} else {Some(Nothing)};
             self.column = brline.len();
           },
           Alphanum(a) if self.keywords.contains(&a) => {
               return_value = Some(Keyword(a));
           },
           _ =>  { return_value = Some(tok); },
       }
       self.column += ind;
       match return_value {
         Some(Nothing) /*if self.current_string.len()>0*/ => {}, // keep looping
         Some(Newline) if !self.keep_newline => {},
         _ => {break;}
       }
     }// do-while loop
     return_value
  }//next
}//impl Iterator for File_tokenizer


// integrate into RustLr:
/*
pub trait Token_translator<AT:Default>
{
  fn translate_token(t:Token)->Lextoken<AT>;
}

impl<AT:Default> Lexer<AT> for File_tokenizer
{
  fn nextsym(&mut self) -> Lextoken<AT>
  {
     Lextoken {
        sym : String::from("num"),
        value : AT::default(),
     }
  }
  fn linenum(&self)-> usize { self.linenum }
}
*/
