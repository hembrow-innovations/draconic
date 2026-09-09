use crate::{TypeAnn, TypeParam};

pub(crate) fn print_type_params(params: &[TypeParam], out: &mut String) {
    if params.is_empty() {
        return;
    }
    out.push('<');
    for (i, p) in params.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(&p.name.name);
    }
    out.push('>');
}

pub(crate) fn print_type_ann(ty: &TypeAnn, out: &mut String) {
    match ty {
        TypeAnn::Named { name, .. } => out.push_str(name),
        TypeAnn::GenericApp { name, args, .. } => {
            out.push_str(name);
            out.push('<');
            for (i, a) in args.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                print_type_ann(a, out);
            }
            out.push('>');
        }
        TypeAnn::Object { props, .. } => {
            out.push_str("{ ");
            for (i, p) in props.iter().enumerate() {
                if i > 0 {
                    out.push_str("; ");
                }
                out.push_str(&p.name);
                out.push_str(": ");
                print_type_ann(&p.ty, out);
            }
            out.push_str(" }");
        }
        TypeAnn::Tuple { elements, .. } => {
            out.push('[');
            for (i, e) in elements.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                print_type_ann(e, out);
            }
            out.push(']');
        }
        TypeAnn::Pointer { inner, .. } => {
            out.push('*');
            print_type_ann(inner, out);
        }
        TypeAnn::Union { types, .. } => {
            for (i, t) in types.iter().enumerate() {
                if i > 0 {
                    out.push_str(" | ");
                }
                print_type_ann(t, out);
            }
        }
        TypeAnn::Intersection { types, .. } => {
            for (i, t) in types.iter().enumerate() {
                if i > 0 {
                    out.push_str(" & ");
                }
                print_type_ann(t, out);
            }
        }
    }
}
