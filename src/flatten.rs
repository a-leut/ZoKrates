//! Module containing the `Flattener` to process a program that it is R1CS-able.
//!
//! @file flatten.rs
//! @author Dennis Kuhnert <dennis.kuhnert@campus.tu-berlin.de>
//! @date 2017

use std::collections::{HashSet, HashMap};
use absy::*;
use absy::Expression::*;
use field::Field;

/// Flattener compute flattened program.
pub struct Flattener {
    /// Number of bits needed to represent the maximum value.
    bits: usize,
    /// Vector containing all used variables while processing the program.
    variables: HashSet<String>,
    /// Map of renamings for reassigned variables while processing the program.
    substitution: HashMap<String, String>,
    /// Index of the next introduced variable while processing the program.
    next_var_idx: usize,
}
impl Flattener {
    /// Returns a `Flattener` with fresh a fresh [substitution] and [variables].
    ///
    /// # Arguments
    ///
    /// * `bits` - Number of bits needed to represent the maximum value.
    pub fn new(bits: usize) -> Flattener {
        Flattener {
            bits: bits,
            variables: HashSet::new(),
            substitution: HashMap::new(),
            next_var_idx: 0
        }
    }

    /// Returns (condition true, condition false) `VariableReference`s for the given condition.
    /// condition true = 1, if `condition` is true, 0 else
    /// condition false = 1, if `condition` is false, 0 else
    ///
    /// # Arguments
    ///
    /// * `statements_flattened` - Vector where new flattened statements can be added.
    /// * `condition` - `Condition` that will be flattened.
    fn flatten_condition<T: Field>(&mut self, statements_flattened: &mut Vec<Statement<T>>, condition: Condition<T>) -> (Expression<T>, Expression<T>) {
        match condition {
            Condition::Lt(lhs, rhs) => {
                let lhs_flattened = self.flatten_expression(statements_flattened, lhs);
                let rhs_flattened = self.flatten_expression(statements_flattened, rhs);

                let lhs_name = format!("sym_{}", self.next_var_idx);
                self.next_var_idx += 1;
                statements_flattened.push(Statement::Definition(lhs_name.to_string(), lhs_flattened));
                let rhs_name = format!("sym_{}", self.next_var_idx);
                self.next_var_idx += 1;
                statements_flattened.push(Statement::Definition(rhs_name.to_string(), rhs_flattened));

                let cond_result = format!("sym_{}", self.next_var_idx);
                self.next_var_idx += 1;
                statements_flattened.push(Statement::Definition(
                    cond_result.to_string(),
                    Sub(
                        box VariableReference(lhs_name.to_string()),
                        box VariableReference(rhs_name.to_string())
                    )
                ));
                for i in 0..self.bits {
                    let new_name = format!("{}_b{}", &cond_result, i);
                    statements_flattened.push(Statement::Definition(
                        new_name.to_string(),
                        Mult(
                            box VariableReference(new_name.to_string()),
                            box VariableReference(new_name.to_string())
                        )
                    ));
                }
                let mut expr = Add(
                    box VariableReference(format!("{}_b0", &cond_result)), // * 2^0
                    box Mult(
                        box VariableReference(format!("{}_b1", &cond_result)),
                        box NumberLiteral(T::from(2))
                    )
                );
                for i in 1..self.bits/2 {
                    expr = Add(
                        box expr,
                        box Add(
                            box Mult(
                                box VariableReference(format!("{}_b{}", &cond_result, 2*i)),
                                box NumberLiteral(T::from(2).pow(i))
                            ),
                            box Mult(
                                box VariableReference(format!("{}_b{}", &cond_result, 2*i+1)),
                                box NumberLiteral(T::from(2).pow(i))
                            ),
                        )
                    );
                }
                expr = Add(
                    box Mult(
                        box VariableReference(format!("{}_b{}", &cond_result, self.bits - 1)),
                        box NumberLiteral(T::zero() - T::from(2).pow(self.bits - 1))
                    ),
                    box expr
                );
                statements_flattened.push(Statement::Definition(cond_result.to_string(), expr));

                let cond_true = format!("{}_b{}", &cond_result, self.bits - 1);
                let cond_false = format!("sym_{}", self.next_var_idx);
                self.next_var_idx += 1;
                statements_flattened.push(Statement::Definition(cond_false.to_string(), Sub(box NumberLiteral(T::one()), box VariableReference(cond_true.to_string()))));
                (VariableReference(cond_true), VariableReference(cond_false))
            },
            Condition::Eq(lhs, rhs) => {
                let name_c = format!("sym_{}", self.next_var_idx);
                self.next_var_idx += 1;
                let name_d = format!("sym_{}", self.next_var_idx);
                self.next_var_idx += 1;
                let name_1_d = format!("sym_{}", self.next_var_idx);
                self.next_var_idx += 1;
                let name_c_d = format!("sym_{}", self.next_var_idx);
                self.next_var_idx += 1;
                let name_w = format!("sym_{}", self.next_var_idx);
                self.next_var_idx += 1;
                // d = {1, if c = 0, 0 else}
                let c = self.flatten_expression(statements_flattened, Sub(box lhs, box rhs));
                statements_flattened.push(Statement::Definition(name_c.to_string(), c));
                statements_flattened.push(Statement::Compiler(name_d.to_string(), IfElse(
                    box Condition::Eq(
                        VariableReference(name_c.to_string()),
                        NumberLiteral(T::zero())
                    ),
                    box NumberLiteral(T::one()),
                    box NumberLiteral(T::zero())
                )));
                statements_flattened.push(Statement::Definition(name_1_d.to_string(), Sub(box NumberLiteral(T::one()), box VariableReference(name_d.to_string()))));
                statements_flattened.push(Statement::Definition(name_c_d.to_string(), Sub(box VariableReference(name_c.to_string()), box VariableReference(name_d.to_string()))));
                // c d = 0, d (1-d) = 0, (c-d)w = 1
                statements_flattened.push(Statement::Compiler(name_w.to_string(), Div(box NumberLiteral(T::one()), box VariableReference(name_c_d.to_string()))));
                statements_flattened.push(Statement::Condition(NumberLiteral(T::zero()), Mult(box VariableReference(name_c), box VariableReference(name_d.to_string()))));
                statements_flattened.push(Statement::Condition(NumberLiteral(T::zero()), Mult(box VariableReference(name_d.to_string()), box VariableReference(name_1_d.to_string()))));
                statements_flattened.push(Statement::Condition(NumberLiteral(T::one()), Mult(box VariableReference(name_c_d), box VariableReference(name_w))));

                (VariableReference(name_d), VariableReference(name_1_d))
            },
            _ => unimplemented!(),
        }
    }

    /// Returns a flattened `Expression` based on the given `expr`.
    ///
    /// # Arguments
    ///
    /// * `statements_flattened` - Vector where new flattened statements can be added.
    /// * `expr` - `Expresstion` that will be flattened.
    fn flatten_expression<T: Field>(&mut self, statements_flattened: &mut Vec<Statement<T>>, expr: Expression<T>) -> Expression<T> {
        match expr {
            x @ NumberLiteral(_) |
            x @ VariableReference(_) => x,
            ref x @ Add(..) |
            ref x @ Sub(..) |
            ref x @ Mult(..) |
            ref x @ Div(..) if x.is_flattened() => x.clone(),
            Add(box left, box right) => {
                let left_flattened = self.flatten_expression(statements_flattened, left);
                let right_flattened = self.flatten_expression(statements_flattened, right);
                let new_left = if left_flattened.is_linear() {
                    left_flattened
                } else {
                    let new_name = format!("sym_{}", self.next_var_idx);
                    self.next_var_idx += 1;
                    statements_flattened.push(Statement::Definition(new_name.to_string(), left_flattened));
                    VariableReference(new_name)
                };
                let new_right = if right_flattened.is_linear() {
                    right_flattened
                } else {
                    let new_name = format!("sym_{}", self.next_var_idx);
                    self.next_var_idx += 1;
                    statements_flattened.push(Statement::Definition(new_name.to_string(), right_flattened));
                    VariableReference(new_name)
                };
                Add(box new_left, box new_right)
            },
            Sub(box left, box right) => {
                let left_flattened = self.flatten_expression(statements_flattened, left);
                let right_flattened = self.flatten_expression(statements_flattened, right);
                let new_left = if left_flattened.is_linear() {
                    left_flattened
                } else {
                    let new_name = format!("sym_{}", self.next_var_idx);
                    self.next_var_idx += 1;
                    statements_flattened.push(Statement::Definition(new_name.to_string(), left_flattened));
                    VariableReference(new_name)
                };
                let new_right = if right_flattened.is_linear() {
                    right_flattened
                } else {
                    let new_name = format!("sym_{}", self.next_var_idx);
                    self.next_var_idx += 1;
                    statements_flattened.push(Statement::Definition(new_name.to_string(), right_flattened));
                    VariableReference(new_name)
                };
                Sub(box new_left, box new_right)
            },
            Mult(box left, box right) => {
                let left_flattened = self.flatten_expression(statements_flattened, left);
                let right_flattened = self.flatten_expression(statements_flattened, right);
                let new_left = if left_flattened.is_linear() {
                    if let Sub(..) = left_flattened {
                        let new_name = format!("sym_{}", self.next_var_idx);
                        self.next_var_idx += 1;
                        statements_flattened.push(Statement::Definition(new_name.to_string(), left_flattened));
                        VariableReference(new_name)
                    } else {
                        left_flattened
                    }
                } else {
                    let new_name = format!("sym_{}", self.next_var_idx);
                    self.next_var_idx += 1;
                    statements_flattened.push(Statement::Definition(new_name.to_string(), left_flattened));
                    VariableReference(new_name)
                };
                let new_right = if right_flattened.is_linear() {
                    if let Sub(..) = right_flattened {
                        let new_name = format!("sym_{}", self.next_var_idx);
                        self.next_var_idx += 1;
                        statements_flattened.push(Statement::Definition(new_name.to_string(), right_flattened));
                        VariableReference(new_name)
                    } else {
                        right_flattened
                    }
                } else {
                    let new_name = format!("sym_{}", self.next_var_idx);
                    self.next_var_idx += 1;
                    statements_flattened.push(Statement::Definition(new_name.to_string(), right_flattened));
                    VariableReference(new_name)
                };
                Mult(box new_left, box new_right)
            },
            Div(box left, box right) => {
                let left_flattened = self.flatten_expression(statements_flattened, left);
                let right_flattened = self.flatten_expression(statements_flattened, right);
                let new_left = if left_flattened.is_linear() {
                    left_flattened
                } else {
                    let new_name = format!("sym_{}", self.next_var_idx);
                    self.next_var_idx += 1;
                    statements_flattened.push(Statement::Definition(new_name.to_string(), left_flattened));
                    VariableReference(new_name)
                };
                let new_right = if right_flattened.is_linear() {
                    right_flattened
                } else {
                    let new_name = format!("sym_{}", self.next_var_idx);
                    self.next_var_idx += 1;
                    statements_flattened.push(Statement::Definition(new_name.to_string(), right_flattened));
                    VariableReference(new_name)
                };
                Div(box new_left, box new_right)
            },
            Pow(base, exponent) => {
                // TODO currently assuming that base is number or variable
                match exponent {
                    box NumberLiteral(ref x) if x > &T::one() => {
                        match base {
                            box VariableReference(ref var) => {
                                let id = if x > &T::from(2) {
                                    let tmp_expression = self.flatten_expression(
                                        statements_flattened,
                                        Pow(
                                            box VariableReference(var.to_string()),
                                            box NumberLiteral(x.clone() - T::one())
                                        )
                                    );
                                    let new_name = format!("sym_{}", self.next_var_idx);
                                    self.next_var_idx += 1;
                                    statements_flattened.push(Statement::Definition(new_name.to_string(), tmp_expression));
                                    new_name
                                } else {
                                    var.to_string()
                                };
                                Mult(
                                    box VariableReference(id.to_string()),
                                    box VariableReference(var.to_string())
                                )
                            },
                            box NumberLiteral(var) => Mult(
                                box NumberLiteral(var.clone()),
                                box NumberLiteral(var)
                            ),
                            _ => panic!("Only variables and numbers allowed in pow base")
                        }
                    }
                    _ => panic!("Expected number > 1 as pow exponent"),
                }
            },
            IfElse(box condition, consequent, alternative) => {
                let (cond_true, cond_false) = self.flatten_condition(statements_flattened, condition);
                // (condition_true * consequent) + (condition_false * alternatuve)
                self.flatten_expression(
                    statements_flattened,
                    Add(
                        box Mult(box cond_true, consequent),
                        box Mult(box cond_false, alternative)
                    )
                )
            },
        }
    }

    /// Returns a flattened `Prog`ram based on the given `prog`.
    ///
    /// # Arguments
    ///
    /// * `prog` - `Prog`ram that will be flattened.
    pub fn flatten_program<T: Field>(&mut self, prog: Prog<T>) -> Prog<T> {
        let mut statements_flattened = Vec::new();
        self.variables = HashSet::new();
        self.substitution = HashMap::new();
        self.next_var_idx = 0;
        for def in prog.statements {
            match def {
                Statement::Return(expr) => {
                    let expr_subbed = expr.apply_substitution(&self.substitution);
                    let rhs = self.flatten_expression(&mut statements_flattened, expr_subbed);
                    self.variables.insert("~out".to_string());
                    statements_flattened.push(Statement::Return(rhs));
                },
                Statement::Definition(id, expr) => {
                    let expr_subbed = expr.apply_substitution(&self.substitution);
                    let rhs = self.flatten_expression(&mut statements_flattened, expr_subbed);
                    statements_flattened.push(Statement::Definition(self.use_variable(id), rhs));
                },
                Statement::Condition(expr1, expr2) => {
                    let expr1_subbed = expr1.apply_substitution(&self.substitution);
                    let expr2_subbed = expr2.apply_substitution(&self.substitution);
                    let (lhs, rhs) = if expr1_subbed.is_linear() {
                        (expr1_subbed, self.flatten_expression(&mut statements_flattened, expr2_subbed))
                    } else if expr2_subbed.is_linear() {
                        (expr2_subbed, self.flatten_expression(&mut statements_flattened, expr1_subbed))
                    } else {
                        unimplemented!()
                    };
                    statements_flattened.push(Statement::Condition(lhs, rhs));
                },
                s @ Statement::Compiler(..) => statements_flattened.push(s),
            }
        }
        Prog { id: prog.id, arguments: prog.arguments, statements: statements_flattened }
    }

    /// Proofs if the given name is a not used variable and returns a fresh variable.
    ///
    /// # Arguments
    ///
    /// * `name` - A String that holds the name of the variable
    fn use_variable(&mut self, name: String) -> String {
        let mut i = 0;
        let mut new_name = name.to_string();
        loop {
            if self.variables.contains(&new_name) {
                new_name = format!("{}_{}", &name, i);
                i += 1;
            } else {
                self.variables.insert(new_name.to_string());
                if i == 1 {
                    self.substitution.insert(name, new_name.to_string());
                } else if i > 1 {
                    self.substitution.insert(format!("{}_{}", name, i - 2), new_name.to_string());
                }
                return new_name;
            }
        }
    }
}