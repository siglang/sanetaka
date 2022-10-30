use crate::{
    error::{CompileError, TypeError, EXPECTED_DATA_TYPE},
    helpers::{compile_block, literal_value},
    ts::{TypeSystem, TypeSystemTrait},
    type_error,
};
use sntk_core::{
    parser::ast::{
        ArrayLiteral, BlockExpression, BooleanLiteral, CallExpression, DataType, Expression, ExpressionStatement, FunctionLiteral, Identifier,
        IfExpression, IndexExpression, InfixExpression, LetStatement, NumberLiteral, ObjectLiteral, PrefixExpression, Program, ReturnStatement,
        Statement, StringLiteral, TypeStatement,
    },
    tokenizer::token::Tokens,
};
use sntk_ir::{
    builtin::get_builtin,
    code::{BinaryOp, BinaryOpEq, Instruction, UnaryOp},
    interpreter::{Interpreter, InterpreterBase},
    value::{LiteralValue, Value},
};

#[derive(Debug, Clone)]
pub struct Code(pub Vec<Instruction>);

impl Code {
    fn new() -> Self {
        Self(Vec::new())
    }

    fn push_instruction(&mut self, instruction: &Instruction) {
        self.0.push(instruction.clone());
    }
}

/// Compile the AST generated by `sntk_core` parser into Sanetaka IR.
#[derive(Debug)]
pub struct Compiler {
    pub program: Program,
    pub code: Code,
}

pub type CompileResult<T> = Result<T, CompileError>;

/// Provides the basic methods of the compiler.
pub trait CompilerTrait {
    fn new(program: Program) -> Self;
    fn compile_program(&mut self) -> CompileResult<Interpreter>;
    fn compile_let_statement(&mut self, let_statement: &LetStatement) -> CompileResult<()>;
    fn compile_return_statement(&mut self, return_statement: &ReturnStatement) -> CompileResult<()>;
    fn compile_type_statement(&mut self, type_statement: &TypeStatement) -> CompileResult<()>;
    fn compile_expression(&mut self, expression: &Expression, data_type: Option<DataType>) -> CompileResult<()>;
}

impl CompilerTrait for Compiler {
    /// **Creates a new Compiler instance.**
    /// it takes an argument of type `Program`.
    fn new(program: Program) -> Self {
        Self { program, code: Code::new() }
    }

    /// Compile the AST generated by `sntk_core` parser into Sanetaka IR.
    fn compile_program(&mut self) -> CompileResult<Interpreter> {
        if !self.program.errors.is_empty() {
            return Err(CompileError::ParsingError(self.program.errors.clone()));
        }

        for statement in self.program.statements.clone() {
            match statement {
                Statement::LetStatement(statement) => self.compile_let_statement(&statement)?,
                Statement::ReturnStatement(statement) => self.compile_return_statement(&statement)?,
                Statement::TypeStatement(statement) => self.compile_type_statement(&statement)?,
                Statement::ExpressionStatement(ExpressionStatement { expression, .. }) => self.compile_expression(&expression, None)?,
            };
        }

        Ok(Interpreter::new(self.code.clone().0))
    }

    /// Compile a `let` statement.
    ///
    /// `let x: number = 5;` to Sanetaka IR:
    /// ```
    /// Instruction:
    ///     0: LoadConst 5.0
    ///     1: StoreName "x"
    /// ```
    fn compile_let_statement(&mut self, let_statement: &LetStatement) -> CompileResult<()> {
        let LetStatement { name, value, data_type, .. } = let_statement;

        self.compile_expression(value, Some(data_type.clone()))?;
        self.code.push_instruction(&Instruction::StoreName(name.clone().value));

        Ok(())
    }

    /// Compile a `return` statement.
    ///
    /// `return 5;` to Sanetaka IR:
    /// ```
    /// Instruction:
    ///     0: LoadConst 5.0
    ///     1: Return
    /// ```
    fn compile_return_statement(&mut self, return_statement: &ReturnStatement) -> CompileResult<()> {
        let ReturnStatement { value, .. } = return_statement;

        self.compile_expression(value, None)?;
        self.code.push_instruction(&Instruction::Return);

        Ok(())
    }

    /// Compile a `type` statement.
    #[allow(unused_variables)]
    fn compile_type_statement(&mut self, type_statement: &TypeStatement) -> CompileResult<()> {
        todo!()
    }

    /// Compile an expression statement.
    fn compile_expression(&mut self, expression: &Expression, data_type: Option<DataType>) -> CompileResult<()> {
        macro_rules! match_type {
            ($type:expr; $e:expr; $pos:expr) => {
                let data_type = data_type.unwrap_or_else(|| TypeSystem::get_data_type_from_expression($e));

                if !TypeSystem(data_type.clone()).eq_from_type(&TypeSystem($type)) {
                    return Err(type_error!(
                        EXPECTED_DATA_TYPE; $type, data_type; $pos;
                    ));
                }
            };
        }

        match expression {
            Expression::BlockExpression(BlockExpression { statements, .. }) => {
                self.code.push_instruction(&Instruction::Block(compile_block(statements.clone())?));

                Ok(())
            }

            Expression::Identifier(Identifier { value, .. }) => {
                self.code.push_instruction(&Instruction::LoadName(value.clone()));

                Ok(())
            }

            Expression::NumberLiteral(NumberLiteral { value, position }) => {
                match_type! { DataType::Number; expression; position.clone() };

                self.code
                    .push_instruction(&Instruction::LoadConst(Value::LiteralValue(LiteralValue::Number(*value))));

                Ok(())
            }

            Expression::StringLiteral(StringLiteral { value, position }) => {
                match_type! { DataType::String; expression; position.clone() };

                self.code
                    .push_instruction(&Instruction::LoadConst(Value::LiteralValue(LiteralValue::String(value.clone()))));

                Ok(())
            }

            Expression::BooleanLiteral(BooleanLiteral { value, position }) => {
                match_type! { DataType::Boolean; expression; position.clone() };

                self.code
                    .push_instruction(&Instruction::LoadConst(Value::LiteralValue(LiteralValue::Boolean(*value))));

                Ok(())
            }

            // TODO: Add type checking for ArrayLiteral, FunctionLiteral, and ObjectLiteral.
            Expression::ArrayLiteral(ArrayLiteral { elements, .. }) => {
                self.code
                    .push_instruction(&Instruction::LoadConst(Value::LiteralValue(LiteralValue::Array(
                        elements.iter().map(|e| literal_value(e.clone())).collect(),
                    ))));

                Ok(())
            }

            Expression::FunctionLiteral(FunctionLiteral { parameters, body, .. }) => {
                let mut statments = Vec::new();

                for statment in body.statements.clone() {
                    if let Statement::ReturnStatement(_) = statment {
                        statments.push(statment);
                        break;
                    } else {
                        statments.push(statment);
                    }
                }

                self.code
                    .push_instruction(&Instruction::LoadConst(Value::LiteralValue(LiteralValue::Function {
                        parameters: parameters.iter().map(|p| p.clone().0.value).collect(),
                        body: compile_block(statments)?,
                    })));

                Ok(())
            }

            Expression::ObjectLiteral(ObjectLiteral { pairs, .. }) => {
                self.code
                    .push_instruction(&Instruction::LoadConst(Value::LiteralValue(LiteralValue::Object(
                        pairs.iter().map(|(k, v)| (k.value.clone(), literal_value(v.clone()))).collect(),
                    ))));

                Ok(())
            }

            Expression::PrefixExpression(PrefixExpression { operator, right, .. }) => {
                self.compile_expression(right, None)?;

                match operator {
                    Tokens::Minus => self.code.push_instruction(&Instruction::UnaryOp(UnaryOp::Minus)),
                    Tokens::Bang => self.code.push_instruction(&Instruction::UnaryOp(UnaryOp::Not)),
                    _ => panic!("Unknown operator: {}", operator),
                }

                Ok(())
            }

            Expression::InfixExpression(InfixExpression { left, operator, right, .. }) => {
                self.compile_expression(left, None)?;
                self.compile_expression(right, None)?;

                match operator {
                    Tokens::Plus => self.code.push_instruction(&Instruction::BinaryOp(BinaryOp::Add)),
                    Tokens::Minus => self.code.push_instruction(&Instruction::BinaryOp(BinaryOp::Sub)),
                    Tokens::Asterisk => self.code.push_instruction(&Instruction::BinaryOp(BinaryOp::Mul)),
                    Tokens::Slash => self.code.push_instruction(&Instruction::BinaryOp(BinaryOp::Div)),
                    Tokens::Percent => self.code.push_instruction(&Instruction::BinaryOp(BinaryOp::Mod)),
                    Tokens::EQ => self.code.push_instruction(&Instruction::BinaryOpEq(BinaryOpEq::Eq)),
                    Tokens::NEQ => self.code.push_instruction(&Instruction::BinaryOpEq(BinaryOpEq::Neq)),
                    Tokens::LT => self.code.push_instruction(&Instruction::BinaryOpEq(BinaryOpEq::Lt)),
                    Tokens::GT => self.code.push_instruction(&Instruction::BinaryOpEq(BinaryOpEq::Gt)),
                    Tokens::LTE => self.code.push_instruction(&Instruction::BinaryOpEq(BinaryOpEq::Lte)),
                    Tokens::GTE => self.code.push_instruction(&Instruction::BinaryOpEq(BinaryOpEq::Gte)),
                    _ => panic!(),
                }

                Ok(())
            }

            Expression::CallExpression(CallExpression { function, arguments, .. }) => {
                for argument in arguments.clone() {
                    self.compile_expression(&argument, None)?;
                }

                match *function.clone() {
                    Expression::Identifier(Identifier { value, .. }) => match get_builtin(value.clone()) {
                        Some(_) => self.code.push_instruction(&Instruction::LoadGlobal(value)),
                        None => self.code.push_instruction(&Instruction::LoadName(value)),
                    },
                    Expression::FunctionLiteral(FunctionLiteral { .. }) | Expression::CallExpression(CallExpression { .. }) => {
                        self.compile_expression(function, None)?;
                    }
                    expression => panic!("Unknown function: {:?}", expression),
                }

                self.code.push_instruction(&Instruction::CallFunction(arguments.len()));

                Ok(())
            }

            Expression::IndexExpression(IndexExpression { .. }) => {
                todo!()
            }

            Expression::IfExpression(IfExpression {
                condition,
                consequence,
                alternative,
                ..
            }) => {
                self.compile_expression(condition, None)?;

                self.code.push_instruction(&Instruction::If(
                    compile_block(consequence.statements.clone())?,
                    alternative
                        .clone()
                        .map(|expression| compile_block(expression.statements.clone()))
                        .transpose()?,
                ));

                Ok(())
            }
        }
    }
}
