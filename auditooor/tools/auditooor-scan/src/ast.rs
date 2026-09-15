//! Solar-backed AST parsing (feature = "solar").
//!
//! A precise replacement for the regex `harness::parse_functions`: it uses
//! Paradigm's `solar-parse` to get real function names, visibility, state
//! mutability and parameter types — handling multi-line headers, modifiers,
//! structs, arrays and overloads that the token scanner approximates. Falls
//! back to the regex parser at the call site when this returns empty/errors.

use crate::harness::FnSig;
use solar_parse::{
    ast,
    interface::{source_map::FileName, ColorChoice, Session},
    Parser,
};

/// Parse `src` into fuzzable entrypoint signatures using the real AST.
/// Returns `Err` only on a hard session failure; a parse with recoverable
/// diagnostics still yields whatever functions were recovered.
pub fn parse_functions_ast(src: &str) -> Result<Vec<FnSig>, String> {
    let sess = Session::builder().with_buffer_emitter(ColorChoice::Never).build();
    let mut result: Vec<FnSig> = Vec::new();

    let _ = sess.enter(|| -> solar_parse::interface::Result<()> {
        let arena = ast::Arena::new();
        let mut parser = Parser::from_source_code(
            &sess,
            &arena,
            FileName::Custom("target.sol".to_string()),
            src.to_string(),
        )?;
        let unit = parser.parse_file().map_err(|e| e.emit())?;
        let sm = sess.source_map();

        for item in unit.items.iter() {
            let ast::ItemKind::Contract(contract) = &item.kind else { continue };
            for member in contract.body.iter() {
                let ast::ItemKind::Function(f) = &member.kind else { continue };
                // Only real `function` definitions (skip constructor/fallback/receive/modifier).
                if f.kind != ast::FunctionKind::Function {
                    continue;
                }
                let vis = f.header.visibility.as_ref().map(|s| s.data);
                let is_entry = matches!(vis, Some(ast::Visibility::External) | Some(ast::Visibility::Public));
                if !is_entry {
                    continue;
                }
                let mutb = f.header.state_mutability.as_ref().map(|s| s.data);
                if matches!(mutb, Some(ast::StateMutability::View) | Some(ast::StateMutability::Pure)) {
                    continue;
                }
                let payable = matches!(mutb, Some(ast::StateMutability::Payable));
                let name = match &f.header.name {
                    Some(id) => id.as_str().to_string(),
                    None => continue,
                };
                let params: Vec<String> = f
                    .header
                    .parameters
                    .vars
                    .iter()
                    .map(|v| {
                        sm.span_to_snippet(v.ty.span)
                            .map(|s| s.split_whitespace().collect::<Vec<_>>().join(" "))
                            .unwrap_or_else(|_| "unknown".to_string())
                    })
                    .collect();
                result.push(FnSig { name, params, payable });
            }
        }
        Ok(())
    });

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ast_ignores_phantoms_in_strings_and_block_comments() {
        // The regex parser conjures functions from a string literal and a /* */
        // block comment and misses the real one. The AST must see only real code.
        let src = r#"
            pragma solidity ^0.8.20;
            contract Phantom {
                string public note = "call function drainEverything() external";
                function deposit() external payable {}
                /* function ghostWithdraw(uint256 amt) external */
                function withdraw(uint256 amt) external {}
            }
        "#;
        let fns = parse_functions_ast(src).unwrap();
        let names: Vec<&str> = fns.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["deposit", "withdraw"], "got {names:?}");
        assert!(fns.iter().find(|f| f.name == "deposit").unwrap().payable);
    }

    #[test]
    fn ast_excludes_view_internal_and_keeps_overloads() {
        let src = r#"
            contract C {
                function a(uint256 x) external {}
                function a(uint256 x, address y) external {}
                function _p() internal {}
                function peek() external view returns (uint256) {}
            }
        "#;
        let fns = parse_functions_ast(src).unwrap();
        assert_eq!(fns.len(), 2, "two overloads of a(), no view/internal: {fns:?}");
        assert!(fns.iter().all(|f| f.name == "a"));
    }
}
