#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(unused_parens)]
#![allow(unused_mut)]
#![allow(unused_assignments)]
#![allow(unused_doc_comments)]
#![allow(unused_imports)]
use regex::Regex;
use std::collections::{HashSet};
use crate::RawToken::*;

/// structure produced by [StrTokenizer].
#[derive(Debug)]
pub enum RawToken<'t>
{
  /// an unsigned integer, though for convenience it is interpreted as
  /// a signed number.  Negative numbers must be recognized by higher-level
  /// parser.  Both decimal and hexadecimal numbers prefixed by 0x are
  /// recognized.
  Num(i64),
//  Hex(u64),
  /// floating point number
  Float(f64),
  /// single character inside single quotes.
  Char(char), 
  /// String literal, allows for nested quotes
  Strlit(&'t str),
  /// Alphanumeric sequence, staring with an alphabetical character or '_',
  /// and followed by arbitrary numbers of alphabetical, numeric or _.
  Alphanum(&'t str),
  /// non-alphanumeric character, either identified as doubles, singles, or
  /// unrecognized sequences.
  Symbol(&'t str),
  /// newline, returned optionally
  Newline,
  /// number of consecutive whitespaces, returned optionally
  Whitespace(usize), // counts number of non-newline whitespaces
  /// usually used to represent comments, if returned optionally
  Verbatim(&'t str),
  /// tokenizer error
  LexError,
}//RawToken


/// Generic str tokenizer that produces [RawToken]s.
pub struct StrTokenizer<'t>
{
   decuint:Regex,
   hexnum:Regex,
   floatp:Regex,
   //strlit:Regex,
   alphan:Regex,
   nonalph:Regex,
   doubles:HashSet<&'t str>,   
   singles:HashSet<char>,
   //other_syms: Vec<&'t str>,
   input: &'t str,
   position: usize,
   /// flag to toggle whether whitespaces should be returned as Whitespace tokens,
   /// default is false.
   pub keep_whitespace:bool,
   /// flag to toggle whether newline characters ('\n') are returned as Newline
   /// tokens. Default is false.  Note that if this flag is set to true then
   /// newline characters are treated differently from other whitespaces.
   /// For example, when parsing languages like Python, both keep_whitespace
   /// and keep_newline should be set to true.  
   pub keep_newline:bool,
   line:usize,
   line_comment:&'t str,
   ml_comment_start:&'t str,
   ml_comment_end:&'t str,
   /// flag to determine if comments are kept and returned as Verbatim tokens,
   /// default is false.
   pub keep_comment:bool,
   line_start:usize, // keep starting position of line, for column info
   src:&'t str,
}
impl<'t> StrTokenizer<'t>
{
  /// creats a new tokenizer with defaults, *does not* set input.
  pub fn new() -> StrTokenizer<'t>
  {
    let decuint = Regex::new(r"^\d+").unwrap();
    let hexnum = Regex::new(r"^0x[\dABCDEFabcdef]+").unwrap();
    let floatp = Regex::new(r"^\d*\x2E\d+").unwrap();
    //let strlit = Regex::new(r"^\x22(?s)(.*?)\x22").unwrap();
    let alphan = Regex::new(r"^[_a-zA-Z][_\da-zA-Z]*").unwrap();
    let nonalph=Regex::new(r"^[!@#$%\^&*\?\-\+\*/\.,<>=~`';:\|\\]+").unwrap();
    let mut doubles = HashSet::with_capacity(16);    
    let mut singles = HashSet::with_capacity(16);
    for c in ['(',')','[',']','{','}'] {singles.insert(c);}
    //let mut other_syms = Vec::with_capacity(32);
    let input = "";
    let position = 0;
    let keep_whitespace=false;
    let keep_newline=false;
    let line = 1;
    let line_comment = "//";
    let ml_comment_start="/*";
    let ml_comment_end="*/";    
    let keep_comment=false;
    let line_start=0;
    let src = "";
    StrTokenizer{decuint,hexnum,floatp,/*strlit,*/alphan,nonalph,doubles,singles,input,position,keep_whitespace,keep_newline,line,line_comment,ml_comment_start,ml_comment_end,keep_comment,line_start,src}
  }// new
  /// adds a symbol of exactly length two. If the length is not two the function
  /// has no effect.  Note that these symbols override all other types except for
  /// leading whitespaces and comments markers, e.g. "//" will have precedence
  /// over "/" and "==" will have precedence over "=".
  pub fn add_double(&mut self, s:&'t str)
  {
    if s.len()==2 { self.doubles.insert(s); }
  }
  /// add a single-character symbol.  The type of the symbol overrides other
  /// types except for whitespaces, comments and double-character symbols.
  pub fn add_single(&mut self, c:char) { self.singles.insert(c);}
  /*
  /// add symbol of length greater than two. Symbols that are prefixes of
  /// other symbols should be added after the longer symbols.
  pub fn add_symbol(&mut self, s:&'t str) {
    if s.len()>2 {self.other_syms.push(s); }
  }
  */
  /// sets the input str to be parsed, resets position information.  Note:
  /// trailing whitespaces are always trimmed from the input.
  pub fn set_input(&mut self, inp:&'t str)
  {
    self.input=inp.trim_end(); self.position=0; self.line=1; self.line_start=0;
  }
  /// sets the symbol that begins a single-line comment. The default is
  /// "//".  If this is set to the empty string then no line-comments are
  /// recognized.
  pub fn set_line_comment(&mut self,cm:&'t str) {
    self.line_comment=cm;
  }
  /// sets the symbols used to delineate multi-line comments using a
  /// whitespace separated string such as "/* */".  These symbols are
  /// also the default.  Set this to the empty string to disable
  /// multi-line comments.
  pub fn set_multiline_comments(&mut self,cm:&'t str)
  {
    if cm.len()==0 {
      self.ml_comment_start=""; self.ml_comment_end=""; return;
    }
    let split:Vec<_> = cm.split_whitespace().collect();
    if split.len()!=2 {return;}
    self.ml_comment_start = split[0].trim();
    self.ml_comment_end = split[1].trim();
  }
  /// the current line that the tokenizer is on
  pub fn line(&self)->usize {self.line}
  /// the current column of the tokenizer
  pub fn column(&self)->usize {self.position-self.line_start+1}
  /// returns the current absolute byte position of the Tokenizer
  pub fn position(&self)-> usize {self.position}
  /// returns the source of the tokenizer such as URL or filename
  pub fn get_source(&self) -> &str {self.src}
  pub fn set_source<'u:'t>(&mut self, s:&'u str) {self.src=s;}
  
  /// returns next token, along with starting line and column numbers.
  /// This function will return None at end of stream or LexError along
  /// with a message printed to stderr if a tokenizer error occured.
  pub fn next_token(&mut self) -> Option<(RawToken<'t>,usize,usize)>
  {
   let mut pi = 0;
   let clen = self.line_comment.len();
   let (cms,cme) = (self.ml_comment_start,self.ml_comment_end);
   while self.position<self.input.len()
   {
    pi = self.position;
    //if pi>=self.input.len() {return None;}
    let mut column0 = self.column();
    let mut line0 = self.line;
    let mut lstart0 = self.line_start;
    
    // skip/keep whitespaces
    let mut nextchars = self.input[pi..].chars();
    let mut c = nextchars.next().unwrap();
    //println!("NEXTCHAR is ({}), position {}",c,self.position);
    let mut i = pi;
    while c.is_whitespace() && i < self.input.len() 
    {
       if c=='\n' {
         self.line+=1; lstart0=self.line_start; self.line_start=i+1; line0=self.line;
         if self.keep_newline { self.position = i+1; return Some((Newline,self.line-1,pi-lstart0+1)); }
       }
       i+= 1; 
       if i<self.input.len() {c = nextchars.next().unwrap();}
    }
    self.position = i;
    if (i>pi && self.keep_whitespace) {
      return Some((Whitespace(i-pi),line0,self.column()-(i-pi)));}
    else if i>pi {continue;}
    //if pi>=self.input.len() {return None;}

    // look for line comment
    if clen>0 && pi+clen<=self.input.len() && self.line_comment==&self.input[pi..pi+clen] {
      if let Some(nlpos) = self.input[pi+clen..].find("\n") {
        self.position = nlpos+pi+clen;
        if self.keep_comment {
          return Some((Verbatim(&self.input[pi..pi+clen+nlpos]),self.line,pi-self.line_start+1));
        }
        else {continue;}
      } else { // no newline fould
        self.position = self.input.len(); 
        if self.keep_comment {return Some((Verbatim(&self.input[pi..]),self.line,pi-self.line_start+1));}
        else {break;}
      }
    }// line comment

    // look for multi-line comment (similar to string literals)
    if cms.len()>0 && pi+cms.len()<=self.input.len() && &self.input[pi..pi+cms.len()] == cms {
       if let Some(endpos) = self.input[pi+cms.len()..].find(cme) {
         self.position = pi+cms.len()+endpos+cme.len();
       } else {
         self.position = self.input.len();
         eprintln!("Tokenizer error: unclosed multi-line comment starting on line {}, column {}",line0,pi-self.line_start+1);
         return Some((LexError,line0,pi-self.line_start+1));
       }
       // find newline chars
       let mut ci = pi;
       while let Some(nli) = self.input[ci..self.position].find('\n')
       {
          self.line+=1; ci += nli+1;  self.line_start=ci;
          // Newline token is never returned if inside string literal
       }
       if self.keep_comment {
         return Some((Verbatim(&self.input[pi..self.position]),line0,pi-lstart0+1));
       }
       else {continue;}
    }//multi-line comments


    // look for doubles
    if pi+1<self.input.len() && self.doubles.contains(&self.input[pi..pi+2]) {
      self.position = pi+2;
      return Some((Symbol(&self.input[pi..pi+2]),self.line,self.column()-2));
    }

    // look for singles:
    //c=self.input[pi..pi+1].chars().next().unwrap();
    if self.singles.contains(&c) {
     // println!("ADDING SINGLE {}",c);
      self.position=pi+1;
      return Some((Symbol(&self.input[pi..pi+1]),self.line,self.column()-1));
    }

    // look for char literal
    if c=='\'' && pi+2<self.input.len() && &self.input[pi+2..pi+3]=="\'" {
      self.position = pi+3;
      let thechar = self.input[pi+1..pi+2].chars().next().unwrap();
      return Some((Char(thechar),self.line,self.column()-3));
    }

    // look for string literal, keep track of newlines
    if c=='\"' {
      let mut ci = pi+1;
      while ci<self.input.len()
      {
         if &self.input[ci..ci+1]=="\"" {
            self.position = ci+1;
            return Some((Strlit(&self.input[pi..self.position]),line0,pi-lstart0+1));
         }
         else if &self.input[ci..ci+1] == "\n" {
           self.line+=1; self.line_start=ci+1;
         }
         // else need to try again!
         else if &self.input[ci..ci+1] == "\\" {ci+=1}; // extra skip
         ci+=1;
      }// while ci < input.len()
      // terminated without finding end of string
      self.position = self.input.len();
        eprintln!("Tokenizer error: unclosed string starting on line {}, column {}",line0,pi-self.line_start+1);
        return Some((LexError,line0,pi-lstart0+1)); 
    }//strlit
    /*
    if let Some(mat) = self.strlit.find(&self.input[pi..]) {
       self.position = mat.end()+pi;
       // find newline chars
       let mut ci = pi;
       while let Some(nli) = self.input[ci..self.position].find('\n')
       {
          self.line+=1; ci += nli+1;  self.line_start=ci;
          // Newline token is never returned if inside string literal
       }
       return Some((Strlit(&self.input[pi..self.position]),line0,pi-lstart0+1));
    }//string lits are matched first, so other's aren't part of strings
    */
    
    // look for hex
    if let Some(mat) = self.hexnum.find(&self.input[pi..]) {
        self.position = mat.end()+pi;
        return Some((Num(i64::from_str_radix(&self.input[pi+2..self.position],16).unwrap()),self.line,pi+3-self.line_start));        
    }//hexnum
    // look for alphanum    
    if let Some(mat) = self.alphan.find(&self.input[pi..]) {
        self.position = mat.end()+pi;  
        return Some((Alphanum(&self.input[pi..self.position]),self.line,pi-self.line_start+1));
    }//alphanums

    // floats
    if let Some(mat) = self.floatp.find(&self.input[pi..]) {
        self.position = mat.end()+pi;
        return Some((Float(self.input[pi..self.position].parse::<f64>().unwrap()),self.line,pi-self.line_start+1));
    }//floatp
    // decimal ints
    if let Some(mat) = self.decuint.find(&self.input[pi..]) {
        self.position = mat.end()+pi;  
        return Some((Num(self.input[pi..self.position].parse::<i64>().unwrap()),self.line,pi-self.line_start+1));
    }//decuint
    //check for unclosed string
    if pi<self.input.len() && &self.input[pi..pi+1]=="\"" {
        self.position = self.input.len();
        eprintln!("Tokenizer error: unclosed string starting on line {}, column {}",line0,pi-self.line_start+1);
        return Some((LexError,line0,pi-self.line_start+1));        
    }//unclosed string
      
    // at this point, what remains must be a recognized, non-alphanumeric symbol
    if let Some(mat) = self.nonalph.find(&self.input[pi..]) {
        self.position = mat.end()+pi;
        return Some((Symbol(&self.input[pi..self.position]),self.line,pi-self.line_start+1));	 
    }

    // at this point, must be error
    self.position = self.input.len();
    if pi<self.position {
      eprintln!("Tokenizer error: unrecognized symbols starting on line {}, column {}",line0,pi-self.line_start+1);
     return Some((LexError,line0,pi-self.line_start+1));
    }
    //else { return None; }
   } //while
   return None;
  }//next_token
  
}//impl StrTokenizer

impl<'t> Iterator for StrTokenizer<'t>
{
  type Item = (RawToken<'t>,usize,usize);
  fn next(&mut self) -> Option<(RawToken<'t>,usize,usize)>
  {
     if let Some(tok) = self.next_token() {Some(tok)} else {None}
  }
}//Iterator


/// Structure to hold contents of a source (such as contents of file).
pub struct LexSource<'t>
{
   pathname:&'t str,
   contents:String,
}
impl<'t> LexSource<'t>
{
  /// creates a new LexSource struct with given source path,
  /// reads contents into struct using [std::fs::read_to_string]
  pub fn new(path:&'t str) -> std::io::Result<LexSource<'t>>
  {
     let tryread=std::fs::read_to_string(path);
     //println!("READTHIS: {:?}",&tryread);
     match tryread {
       Ok(st) => {
         Ok(LexSource {
           pathname:path,
           contents:st,
         })
       },
       Err(e) => {Err(e)}
     }//match
  }//new
//  /// sets the numerical id of this source.
//  pub fn set_id(&mut self, id:usize) {self.id=id;}
//  pub fn get_id(&self)->usize {self.id}
  /// retrieves entire contents of lexsource
  pub fn get_contents(&self)->&str {&self.contents}
  /// retrieves original path (such as filename) of this source
  pub fn get_path(&self)->&str {self.pathname}
}//impl LexSource
impl<'t> StrTokenizer<'t>
{
   /// creates a StrTokenizer from a [LexSource] structure that contains
   /// a string representing the contents of the source, and
   /// calls [StrTokenizer::set_input] to reference that string.
   /// The proper way to create a tokenizer that reads from a file is therefore:
   ///   ```ignore
   ///   let source = LexSource::new(source_path).unwrap();
   ///   let mut tokenizer = StrTokenizer::from_source(&source);
   ///   ```
   pub fn from_source(ls:&'t LexSource<'t>) ->StrTokenizer<'t>
   {
      let mut stk = StrTokenizer::new();
      stk.set_input(ls.contents.as_str());
      stk
   }
   /// creates a string tokenizer and sets input to give str.
   pub fn from_str(s:&'t str) -> StrTokenizer<'t>
   {
      let mut stk = StrTokenizer::new();
      stk.set_input(s);
      stk
   }
}// impl StrTokenizer
