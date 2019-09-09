
#[derive(Clone)]
pub struct ProgramBranch {
    pub field_lens: Vec<usize>,
    pub program: Vec<Instruction>,
}

// could make this a single u16 fairly easily
#[derive(Clone)]
pub enum Instruction {
    // obviously flatten this
    ReadSlice { ident: usize, branches: Vec<ProgramBranch> },
    WriteSlice(usize),
    WriteConstant(u8),
}

#[derive(Clone, Copy)]
pub struct Slice { pub start: usize, pub len: usize }

// struct Stack<'a> { data: Vec<Slice>, prev: Option<&'a Stack> }
// would we even use this if we want to avoid recursion anyway?


pub fn execute_byte(
    program: &Vec<Instruction>,
    data: &Vec<u8>,
) -> u8 {
    let result = execute(program, data);
    assert!(result.len() == 1, "Execute byte expects 1 byte");
    result[0]
}

pub fn execute(
    program: &Vec<Instruction>,
    data: &Vec<u8>,
) -> Vec<u8> {
    let mut result = Vec::new();
    let variables = vec![Slice { start: 0, len: data.len() }];
    execute_with(
        program,
        data,
        variables,
        &mut result,
    );
    return result;
}

fn execute_with(
    program: &Vec<Instruction>,
    data: &Vec<u8>,
    variables: Vec<Slice>,
    out: &mut Vec<u8>,
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
                execute_with(program, data, variables, out);
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

    #[test]
    fn test() {
        use super::Instruction::*;
        assert_eq!(execute(&vec![WriteConstant(33)], &vec![]), vec![33]);
        assert_eq!(execute(&vec![WriteSlice(0)], &vec![0, 1, 2]), vec![0, 1, 2]);
        {
            let prog = vec![ReadSlice{ident: 0, branches: vec![
                ProgramBranch { field_lens: vec![], program: vec![WriteConstant(0)] },
                ProgramBranch { field_lens: vec![1], program: vec![WriteSlice(1)] },
            ]}];
            assert_eq!(execute(&prog, &vec![0]), vec![0]);
            assert_eq!(execute(&prog, &vec![1, 25]), vec![25]);
        }
    }
}

