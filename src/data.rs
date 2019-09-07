
#[derive(Clone)]
struct ProgramBranch {
    field_lens: Vec<usize>,
    program: Vec<Instruction>,
}

// could make this a single u16 fairly easily
#[derive(Clone)]
enum Instruction {
    // obviously flatten this
    ReadSlice { ident: usize, branches: Vec<ProgramBranch> },
    WriteSlice(usize),
    WriteConstant(u8),
}

#[derive(Clone, Copy)]
struct Slice { start: usize, len: usize }

// struct Stack<'a> { data: Vec<Slice>, prev: Option<&'a Stack> }
// would we even use this if we want to avoid recursion anyway?

fn execute(
    data: &Vec<u8>,
    variables: Vec<Slice>,
    out: &mut Vec<u8>,
    program: &Vec<Instruction>,
    // pc: usize,
) {
    use self::Instruction::*;
    for instruction in program {
        match instruction {
            &ReadSlice { ident, ref branches } => {
                let Slice { mut start, .. } = variables[ident];
                let disc = data[start] as usize;
                start += 1;
                let ProgramBranch { field_lens, program } = &branches[disc];
                // @Performance capacity
                let mut variables = variables.clone();
                for &len in field_lens {
                    variables.push(Slice { start, len });
                    start += len;
                }
                execute(data, variables, out, program);
            },
            &WriteSlice(ident) => {
                let Slice { start, len } = variables[ident];
                for i in 0..len {
                    out.push(data[start + i]);
                }
            },
            &WriteConstant(val) => {
                out.push(val);
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! evaluates {
        { $prog: expr, $data: expr => $expected: expr } => {
            let prog: Vec<Instruction> = $prog;
            let data: Vec<u8> = $data;
            let expected: Vec<u8> = $expected;
            let all = Slice { start: 0, len: data.len() };
            let mut actual = Vec::new();
            execute(
                &data,
                vec![all],
                &mut actual,
                &prog,
            );
            assert_eq!(expected, actual);
        }
    }

    #[test]
    fn test() {
        use super::Instruction::*;
        evaluates!{ vec![WriteConstant(33)], vec![] => vec![33] }
        evaluates!{ vec![WriteSlice(0)], vec![0, 1, 2] => vec![0, 1, 2] }
        {
            let prog = vec![ReadSlice{ident: 0, branches: vec![
                ProgramBranch { field_lens: vec![], program: vec![WriteConstant(0)] },
                ProgramBranch { field_lens: vec![1], program: vec![WriteSlice(1)] },
            ]}];
            evaluates!{ prog.clone(), vec![0] => vec![0] }
            evaluates!{ prog.clone(), vec![1, 25] => vec![25] }
        }
    }
}

