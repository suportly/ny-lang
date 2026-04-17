import sys
import re

def modify_resolver():
    with open('src/semantic/resolver.rs', 'r') as f:
        content = f.read()

    # 1. Add find_similar_struct
    if "fn find_similar_struct" not in content:
        find_similar_struct_code = """
    fn find_similar_struct(&self, name: &str) -> Option<String> {
        let mut best: Option<(String, usize)> = None;
        for key in self.structs.keys() {
            let dist = crate::common::edit_distance(name, key);
            if dist <= 2 && dist < name.len() && best.as_ref().map_or(true, |(_, d)| dist < *d) {
                best = Some((key.clone(), dist));
            }
        }
        best.map(|(name, _)| name)
    }
"""
        content = content.replace("fn resolve_type_annotation", find_similar_struct_code.strip() + "\n\n    fn resolve_type_annotation")

    # 2. Update unknown variable in resolve_expr (Expr::Identifier)
    # The existing code is likely:
    # self.errors.push(CompileError::name_error(
    #     format!("unknown variable '{}'", name),
    #     *span,
    # ));
    # or similar.
    
    # Let's search for exact strings in content to replace.
    var_err_match = re.search(r'self\.errors\.push\(\s*CompileError::name_error\(\s*format!\("unknown variable \'{}\'", name\),\s*\*span,?\s*\),?\s*\);', content)
    if var_err_match:
        replacement = """let mut err = CompileError::name_error(
                    format!("unknown variable '{}'", name),
                    *span,
                );
                if let Some(similar) = self.find_similar_name(name) {
                    err = err.with_note(format!("did you mean '{}'?", similar));
                }
                self.errors.push(err);"""
        content = content[:var_err_match.start()] + replacement + content[var_err_match.end():]
    else:
        print("Could not find unknown variable error to replace.")

    # 3. Update unknown struct in resolve_expr (Expr::StructInit)
    struct_err_match = re.search(r'self\.errors\.push\(\s*CompileError::name_error\(\s*format!\("unknown struct \'{}\'", name\),\s*\*span,?\s*\),?\s*\);', content)
    if struct_err_match:
        replacement = """let mut err = CompileError::name_error(
                    format!("unknown struct '{}'", name),
                    *span,
                );
                if let Some(similar) = self.find_similar_struct(name) {
                    err = err.with_note(format!("did you mean '{}'?", similar));
                }
                self.errors.push(err);"""
        content = content[:struct_err_match.start()] + replacement + content[struct_err_match.end():]
    else:
        print("Could not find unknown struct error to replace.")

    # 4. Update immutable assignment in resolve_stmt (Stmt::Assignment)
    # Something like:
    # if symbol.mutability == Mutability::Immutable {
    #     self.errors.push(CompileError::immutability(...));
    # }
    # Let's look for assignment to immutable
    assign_err_match = re.search(r'self\.errors\.push\(\s*CompileError::type_error\(\s*format!\("cannot assign to immutable variable \'{}\'", name\),\s*\*span,?\s*\),?\s*\);', content)
    if assign_err_match:
        replacement = """self.errors.push(CompileError::type_error(
                            format!("cannot assign twice to immutable variable '{}'", name),
                            *span,
                        ));"""
        content = content[:assign_err_match.start()] + replacement + content[assign_err_match.end():]
    else:
        # Maybe it's immutability error
        assign_err_match2 = re.search(r'self\.errors\.push\(\s*CompileError::immutability\(\s*format!\("cannot assign to immutable variable \'{}\'", name\),\s*\*span,?\s*\),?\s*\);', content)
        if assign_err_match2:
            replacement = """self.errors.push(CompileError::immutability(
                                format!("cannot assign twice to immutable variable '{}'", name),
                                *span,
                            ));"""
            content = content[:assign_err_match2.start()] + replacement + content[assign_err_match2.end():]
        else:
            print("Could not find immutable assignment error to replace.")

    # Add tests
    tests = """
#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::Span;
    use crate::parser::ast::{Expr, Stmt, TypeAnnotation, Mutability};

    // Helper to get a dummy span
    fn dummy_span() -> Span {
        Span { start: 0, end: 0, line: 1, column: 1 }
    }

    #[test]
    fn test_resolve_unknown_variable_with_suggestion() {
        let mut resolver = Resolver::new();
        resolver.declare("my_variable", Symbol {
            name: "my_variable".to_string(),
            ty: NyType::I32,
            mutability: Mutability::Immutable,
            span: dummy_span(),
        });
        
        let expr = Expr::Identifier { name: "my_varibale".to_string(), span: dummy_span() };
        resolver.resolve_expr(&expr);
        
        assert_eq!(resolver.errors.len(), 1);
        assert!(resolver.errors[0].message.contains("unknown variable 'my_varibale'"));
        assert_eq!(resolver.errors[0].notes.len(), 1);
        assert!(resolver.errors[0].notes[0].contains("did you mean 'my_variable'?"));
    }

    #[test]
    fn test_resolve_unknown_struct_with_suggestion() {
        let mut resolver = Resolver::new();
        resolver.structs.insert("MyStruct".to_string(), vec![]);
        
        let expr = Expr::StructInit { name: "MyStrcut".to_string(), fields: vec![], span: dummy_span() };
        resolver.resolve_expr(&expr);
        
        assert_eq!(resolver.errors.len(), 1);
        assert!(resolver.errors[0].message.contains("unknown struct 'MyStrcut'"));
        assert_eq!(resolver.errors[0].notes.len(), 1);
        assert!(resolver.errors[0].notes[0].contains("did you mean 'MyStruct'?"));
    }

    #[test]
    fn test_resolve_immutable_assignment_error() {
        let mut resolver = Resolver::new();
        resolver.declare("x", Symbol {
            name: "x".to_string(),
            ty: NyType::I32,
            mutability: Mutability::Immutable,
            span: dummy_span(),
        });
        
        let stmt = Stmt::Assignment { 
            target: Box::new(Expr::Identifier { name: "x".to_string(), span: dummy_span() }),
            value: Box::new(Expr::Literal { value: Literal::Int(42), span: dummy_span() }),
            span: dummy_span()
        };
        resolver.resolve_stmt(&stmt);
        
        assert_eq!(resolver.errors.len(), 1);
        assert!(resolver.errors[0].message.contains("cannot assign twice to immutable variable 'x'"));
    }
}
"""
    if "mod tests {" not in content:
        content += tests

    with open('src/semantic/resolver.rs', 'w') as f:
        f.write(content)

modify_resolver()
