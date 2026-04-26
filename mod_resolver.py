import sys

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
        # Insert it before resolve_type_annotation
        content = content.replace("fn resolve_type_annotation", find_similar_struct_code.strip() + "\n\n    fn resolve_type_annotation")

    # 2. Update unknown variable in resolve_expr (Expr::Identifier)
    # We need to replace:
    # self.errors.push(CompileError::name_error(
    #     format!("unknown variable '{}'", name),
    #     *span,
    # ));
    # with:
    # let mut err = CompileError::name_error(format!("unknown variable '{}'", name), *span);
    # if let Some(similar) = self.find_similar_name(name) {
    #     err = err.with_note(format!("did you mean '{}'?", similar));
    # }
    # self.errors.push(err);
    
    pattern1 = """self.errors.push(CompileError::name_error(
                    format!("unknown variable '{}'", name),
                    *span,
                ));"""
    replacement1 = """let mut err = CompileError::name_error(
                    format!("unknown variable '{}'", name),
                    *span,
                );
                if let Some(similar) = self.find_similar_name(name) {
                    err = err.with_note(format!("did you mean '{}'?", similar));
                }
                self.errors.push(err);"""
    
    # Let's try a regex or simpler replace
    if "unknown variable" in content:
        # manual replace
        pass

    # 3. Update unknown struct in resolve_expr (Expr::StructInit)
    # 4. Update immutable assignment in resolve_stmt (Stmt::Assignment)

    with open('src/semantic/resolver_new.rs', 'w') as f:
        f.write(content)

modify_resolver()
