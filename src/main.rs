#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(unused_parens)]
#![allow(unused_mut)]
#![allow(unused_imports)]
use std::collections::{HashMap,HashSet};
mod lib;
use lib::*;
use lib::Token::*;

// hand-build a lexer
fn main()
{
  let input = "x=12 &&u2*20==.14 -> \"ab c \" + k; print(f); n.f(); #b l a h";

  let mut scanner = Str_tokenizer::new(input);
  let tokens:Vec<_> = scanner.collect();
  println!("tokens: {:?}",tokens);
  //for t in scanner {println!("{:?}",t);}

  println!("-----------------");

  // testing File_tokenizer
  let mut fscan = File_tokenizer::new("test.c");
  fscan.add_keywords("while return");
  fscan.set_line_comment("//");
  fscan.set_keep_comments(false);
  fscan.set_keep_newline(true);  
  while let Some(token) = fscan.next() {
    println!("{:?}, line {}",&token,fscan.line_number());  
  }
//  for t in fscan {println!("{:?}",t);}
//  println!("line number: {}",fscan.line_number);

  let mut scanner = Str_tokenizer::new("while (1+1==2) fork();");
   while let Some(token) = scanner.next() {
     println!("{:?}",&token);
   }
}//main
