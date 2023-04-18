// syntax (like BNF)
//
// root      = concat
// concat    = ( group | set | repeat | union | position | matcher )+
// group     = '(' concat ')'
// set       = '[' set-items ']'
// set-items = ( char | char '-' char )+
// repeat    = repeat_g | repeat_ng
// repeat_g  = term '{' number '}'              == term{n, n}
//           | term '{' number ',' '}           == term{n, inf}
//           | term '{' ',' number '}'          == term{0, n}
//           | term '{' number ',' number '}'   == term{n, m}
//           | term '*'                         == term{0, inf}
//           | term '+'                         == term{1, inf}
//           | term '?'                         == term{0, 1}
// repeat_ng = repeat_g '?'
// union     = concat '|' concat
// position  = '^' | '$'
// matcher   = '\' meta-char | char

pub(crate) mod ast;
mod parser;

#[cfg(test)]
mod tests;

pub(crate) use ast::Ast;
pub(crate) use parser::Parser;
