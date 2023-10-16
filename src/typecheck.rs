
use std::{convert::identity, iter};

use crate::{codegen::{InstrKind, BrLabel, FnLabel, Producer, Instr}, parser::{Type, literal_type, Span}, arch::Intrinsic, diagnostic::Diagnostic};

pub(crate) fn typecheck<I: Intrinsic>(bytecode: Vec<Instr<I>>) -> Result<(), TypeError> {

    let main = match find_fn_label(&bytecode, &FnLabel::new("main".to_string())) {
        Some(val) => val,
        None => return Err(TypeError::unspanned(TypeErrorKind::MissingMain))
    };

    let mut stack = Vec::new();
    eval_fn(&bytecode, &mut stack, main, InstrKind::Return)?;

    if !stack.is_empty() {
        return Err(TypeError::unspanned(TypeErrorKind::InvalidMain { got: stack }))
    }

    Ok(())

}

fn eval_fn<I: Intrinsic>(bytecode: &Vec<Instr<I>>, stack: &mut Vec<Type>, start: usize, end: InstrKind<I>) -> Result<(), TypeError> {

    let mut file_name = "{unknown}";
    let mut ip = start;

    loop {

        let instr = &bytecode[ip];

        if &instr.kind == &end {
            return Ok(())
        }

        if let InstrKind::FileStart { name } = &instr.kind {
            file_name = name;
        }

        match eval_instr(bytecode, stack, &mut ip, &instr.kind) {
            Ok(()) => (),
            Err(mut err) => {
                err.file_span = FileSpan {
                    span: instr.span,
                    file: file_name.to_string(),
                };
                return Err(err)
            }
        };

        ip += 1;

    }

}

pub(crate) fn eval_instr<I: Intrinsic>(bytecode: &Vec<Instr<I>>, stack: &mut Vec<Type>, ip: &mut usize, instr_kind: &InstrKind<I>) -> Result<(), TypeError> {

    match instr_kind {

        InstrKind::Push { value } => {
            stack.push(literal_type(value))
        },

        InstrKind::Call { to } => { // todo: use fn signature
            let position = match find_fn_label(&bytecode, to) {
                Some(val) => val,
                None => return Err(TypeError::unspanned(TypeErrorKind::UnknownFn { name: to.inner.clone() }))
            };
            eval_fn(bytecode, stack, position, InstrKind::Return)?;
        },
        InstrKind::Return => (),

        InstrKind::Drop => {
            if let Some(val) = stack.pop() {
                drop(val)
            } else {
                return Err(TypeError::unspanned(TypeErrorKind::Mismatch { want: vec![Type::Any], got: stack.to_vec() }))
            }
        },
        InstrKind::Copy => {
            if let Some(val) = stack.pop() {
                stack.push(val.clone());
                stack.push(val.clone());
            } else {
                return Err(TypeError::unspanned(TypeErrorKind::Mismatch { want: vec![Type::Any], got: stack.to_vec() }))
            }
        },
        InstrKind::Over => {
            let got = split_signature::<2>(stack);
            if let [Some(first), Some(second)] = got {
                stack.push(first.clone());
                stack.push(second.clone());
                stack.push(first.clone());
            } else {
                return Err(TypeError::unspanned(TypeErrorKind::Mismatch { want: vec![Type::Any; 2], got: stack.to_vec() }))
            }
        },
        InstrKind::Swap => {
            let got = split_signature::<2>(stack);
            if let [Some(first), Some(second)] = got {
                stack.push(second);
                stack.push(first);
            } else {
                return Err(TypeError::unspanned(TypeErrorKind::Mismatch { want: vec![Type::Any; 2], got: stack.to_vec() }))
            }
        },
        InstrKind::Rot3 => {
            let len = stack.len();
            let elems = match stack.get_mut(len - 4..) {
                Some(val) => val,
                None => return Err(TypeError::unspanned(TypeErrorKind::Mismatch { want: vec![Type::Any; 3], got: stack.to_vec() }))
            };
            elems.rotate_left(1);
        },
        InstrKind::Rot4 => {
            let len = stack.len();
            let elems = match stack.get_mut(len - 5..) {
                Some(val) => val,
                None => return Err(TypeError::unspanned(TypeErrorKind::Mismatch { want: vec![Type::Any; 4], got: stack.to_vec() }))
            };
            elems.rotate_left(1);
        },

        InstrKind::Read => {
            match stack.pop() {
                Some(Type::Ptr { inner }) => {
                    stack.push(*inner);
                },
                other => return Err(TypeError::unspanned(TypeErrorKind::Mismatch { want: vec![Type::Ptr { inner: Box::new(Type::Any) }], got: other.into_iter().collect() }))
            }
        },
        InstrKind::Write => {
            let got = split_signature::<2>(stack);
            match got { // todo: rewrtie thi
                [Some(Type::Ptr { ref inner }), Some(ref value)] => {
                    if inner.as_ref() != value {
                        return Err(TypeError::unspanned(TypeErrorKind::Mismatch { want: vec![Type::Ptr { inner: Box::new(value.clone()) }, Type::Any], got: got.into_iter().filter_map(identity).collect() }))
                    };
                    stack.push(value.clone());
                },
                other => return Err(TypeError::unspanned(TypeErrorKind::Mismatch { want: vec![Type::Ptr { inner: Box::new(Type::Any) }, Type::Any], got: other.into_iter().filter_map(identity).collect() }))
            }
        },
        // Move,

        // Addr,
        // Type,
        // Size,

        // Access,

        InstrKind::Add => {
            let got = split_signature::<2>(stack);
            verify_signature([Type::Int, Type::Int], got)?;
            stack.push(Type::Int);
        },
        InstrKind::Sub => {
            let got = split_signature::<2>(stack);
            verify_signature([Type::Int, Type::Int], got)?;
            stack.push(Type::Int);
        },
        InstrKind::Mul => {
            let got = split_signature::<2>(stack);
            verify_signature([Type::Int, Type::Int], got)?;
            stack.push(Type::Int);
        },
        InstrKind::Dvm => {
            let got = split_signature::<2>(stack);
            verify_signature([Type::Int, Type::Int], got)?;
            stack.push(Type::Int);
        },

        InstrKind::Not => {
            let got = split_signature::<1>(stack);
            verify_signature([Type::Bool], got)?;
            stack.push(Type::Bool);
        },
        InstrKind::And => {
            let got = split_signature::<2>(stack);
            verify_signature([Type::Bool, Type::Bool], got)?;
            stack.push(Type::Bool);
        },
        InstrKind::Or  => {
            let got = split_signature::<2>(stack);
            verify_signature([Type::Bool, Type::Bool], got)?;
            stack.push(Type::Bool);
        },
        InstrKind::Xor => {
            let got = split_signature::<2>(stack);
            verify_signature([Type::Bool, Type::Bool], got)?;
            stack.push(Type::Bool);
        },
                
        InstrKind::Eq  => {
            let got = split_signature::<2>(stack);
            verify_signature([Type::Int, Type::Int], got)?;
            stack.push(Type::Bool);
        },
        InstrKind::Gt  => {
            let got = split_signature::<2>(stack);
            verify_signature([Type::Int, Type::Int], got)?;
            stack.push(Type::Bool);
        },
        InstrKind::Gte => {
            let got = split_signature::<2>(stack);
            verify_signature([Type::Int, Type::Int], got)?;
            stack.push(Type::Bool);
        },
        InstrKind::Lt  => {
            let got = split_signature::<2>(stack);
            verify_signature([Type::Int, Type::Int], got)?;
            stack.push(Type::Bool);
        },
        InstrKind::Lte => {
            let got = split_signature::<2>(stack);
            verify_signature([Type::Int, Type::Int], got)?;
            stack.push(Type::Bool);
        },

        InstrKind::Bne { to } => {

            let got = split_signature::<1>(stack);
            verify_signature([Type::Bool], got)?;

            let position = find_br_label(bytecode, to).expect("invalid br-label");
            let else_bra = &bytecode[position - 1]; // the bra from the `else` should be there

            if let InstrKind::Bra { to: else_to } = &else_bra.kind {
                // `if/else` block
                let mut if_stack = stack.clone();
                let mut else_stack = stack.clone();
                eval_fn(bytecode, &mut if_stack, *ip + 1, InstrKind::BrLabel { label: *to, producer: Producer::If })?;
                eval_fn(bytecode, &mut else_stack, position, InstrKind::BrLabel { label: *else_to, producer: Producer::Else })?;
                if if_stack != else_stack {
                    return Err(TypeError::unspanned(TypeErrorKind::BranchesNotEqual));
                }
                *stack = if_stack;
                let end_position = find_br_label(bytecode, else_to).expect("invalid br-label");
                *ip = end_position; // remember: ip will be incremented again when we return here
            } else {
                // `if` block
                let mut if_stack = stack.clone();
                eval_fn(bytecode, &mut if_stack, *ip + 1, InstrKind::BrLabel { label: *to, producer: Producer::If })?;
                if stack != &mut if_stack {
                    return Err(TypeError::unspanned(TypeErrorKind::BranchesNotEmpty));
                }
                *ip = position; // remember: ip will be incremented again when we return here
            }

        },

        InstrKind::BrLabel { label, producer: Producer::Loop } => {

            let mut loop_stack = stack.clone();
            eval_fn(bytecode, &mut loop_stack, *ip + 1, InstrKind::Bra { to: *label })?;
            if stack != &mut loop_stack {
                return Err(TypeError::unspanned(TypeErrorKind::BranchesNotEmpty));
            }
            let pos = bytecode.iter().position(|item| { // todo: wtf
                if let InstrKind::Bra { to: this } = &item.kind {
                    this == label
                } else {
                    false
                }
            }).expect("loop bra not found");
            *ip = pos; // remember: ip will be incremented again when we return here

        },

        InstrKind::Intrinsic(intrinsic) => {
            I::signature(intrinsic);
        },

        InstrKind::Bra { .. } => (),
        InstrKind::FileStart { .. } => (),
        InstrKind::FnLabel { .. } => (),
        InstrKind::BrLabel { .. } => (),

    };

    Ok(())

}

pub(crate) fn split_signature<const N: usize>(vec: &mut Vec<Type>) -> [Option<Type>; N] {
    const NONE: Option<Type> = None;
    let mut res = [NONE; N];
    let start = if vec.len() < N { vec.len() - 1 } else { vec.len() - N };
    for (idx, item) in vec.drain(start..).rev().enumerate() {
        res[idx] = Some(item);
    }
    res
}

pub(crate) fn verify_signature<const N: usize>(want: [Type; N], got: [Option<Type>; N]) -> Result<(), TypeError> {
    for (lhs, rhs) in iter::zip(&want, &got) {
        let expect_any   = lhs == &Type::Any && rhs.is_none();
        let expect_match = lhs != &Type::Any && Some(lhs) != rhs.as_ref();
        if expect_any || expect_match {
            return Err(TypeError::unspanned(TypeErrorKind::Mismatch { want: want.to_vec(), got: got.into_iter().filter_map(identity).collect() }))
        }
    }
    Ok(())
}

fn find_fn_label<I: Intrinsic>(bytecode: &Vec<Instr<I>>, target: &FnLabel) -> Option<usize> {
    bytecode.iter().position(|item| {
        if let InstrKind::FnLabel { label, .. } = &item.kind {
            label == target
        } else {
            false
        }
    })
}

fn find_br_label<I: Intrinsic>(bytecode: &Vec<Instr<I>>, target: &BrLabel) -> Option<usize> {
    bytecode.iter().position(|item| {
        if let InstrKind::BrLabel { label, .. } = &item.kind {
            label == target
        } else {
            false
        }
    })
}

#[derive(Debug, Default)]
pub(crate) struct FileSpan {
    span: Span,
    file: String,
}

impl FileSpan {
    pub(crate) fn new(span: Span, file: &str) -> Self {
        Self { span, file: file.to_string() }
    }
}

pub(crate) struct TypeError {
    kind: TypeErrorKind,
    file_span: FileSpan,
}

impl TypeError {
    pub(crate) fn unspanned(kind: TypeErrorKind) -> Self {
        Self { kind, file_span: FileSpan::default() }
    }
}

pub(crate) enum TypeErrorKind {
    MissingMain,
    InvalidMain { got: Vec<Type> },
    UnknownFn { name: String },
    BranchesNotEmpty,
    BranchesNotEqual,
    Mismatch { want: Vec<Type>, got: Vec<Type> }
}

pub(crate) fn format_error(value: TypeError) -> Diagnostic {
    let diag = match &value.kind {
        TypeErrorKind::MissingMain => Diagnostic::error("missing `main` function"),
        TypeErrorKind::InvalidMain { got } => Diagnostic::error("invalid `main` function").note(format!("got {:?}", got)),
        TypeErrorKind::UnknownFn { name } => Diagnostic::error("unknown function").code(name),
        TypeErrorKind::BranchesNotEmpty => {
            Diagnostic::error("branch changes stack")
                .note("this branch may not change the types on the stack")
                .note("use `if/else` instead")
        },
        TypeErrorKind::BranchesNotEqual => {
            Diagnostic::error("branches not equal")
                .note("both branches have to evaluate to the same types")
        },
        TypeErrorKind::Mismatch { want, got } => {
            Diagnostic::error("type mismatch")
                .note(format!("want: {:?}", want))
                .note(format!("got: {:?}", got))
        }
    };
    if value.file_span.span != Span::default() {
        diag.file(&value.file_span.file)
            .pos(value.file_span.span.to_pos())
    } else {
        diag
    }
}

