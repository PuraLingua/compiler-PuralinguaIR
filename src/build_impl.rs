use std::{
    collections::{BTreeMap, HashMap},
    range::Range,
};

use ast::{identifier::Identifier, method::ArrayLen};
use compiler_base::{
    BuildOptions, ITypeQuery, TypeQueryOptions,
    abstract_info::{
        IToAbstract, type_information::TypeInformation, type_reference::TypeReference,
    },
};
use compiler_service::ICompilerService;
use pura_lingua::{
    binary::{
        self,
        assembly::AssemblyBuilder,
        prelude::{
            MethodToken, MethodTokenBuilder, MethodType, TypeToken, TypeTokenBuilder, TypeType,
        },
        ty::{InterfaceImplementation, MethodSpec, TypeRef, TypeSpec},
    },
    global::{
        self,
        instruction::{
            CommonReadPointerTo, CommonWritePointer, IRegisterAddr, Instruction, Instruction_Call,
            Instruction_CommonCheck, Instruction_Jump, Instruction_Load, Instruction_New,
            Instruction_Set, JumpCondition, JumpTargetBuilder, LoadContent, RegisterAddr,
            ToCheckContent,
        },
    },
};

use crate::Compiler;

#[derive(thiserror::Error, derive_more::Display, Debug)]
pub enum BuildError {
    #[display("Type {_0:?} not found")]
    TypeNotFound(TypeReference),
    #[display("Field `{_0:?}` not found")]
    FieldNotFound(ast::field::FieldReference),
    #[display("Method `{_0:?}` not found")]
    MethodNotFound(ast::method::MethodReference),
    #[display("Variable `{_0}` not found")]
    VarNotFound(String),
}

impl Compiler {
    pub(super) fn build_impl(
        &self,
        assembly: &mut AssemblyBuilder,
        #[allow(unused)] options: &BuildOptions,
    ) -> global::Result<()> {
        let file = self.file.get_cloned().unwrap();
        #[cfg(debug_assertions)]
        let last_index = self.last_index.get_cloned().unwrap().unwrap_or_else(|| {
            println!("Last index has not been set, use default");
            0
        });

        for (ind, ty) in file.types.iter().enumerate() {
            debug_assert_eq!((ind as u32) + last_index, ty.index());

            match ty {
                ast::ty::TypeDef::Class(class_def) => {
                    let mut override_count = 0;
                    let mut interface_implementations = HashMap::new();
                    let ty_info = ty.to_abstract().into_ok();
                    let marshaled = binary::ty::ClassDef {
                        main: class_def.main,
                        name: assembly.add_string(class_def.name.as_ref()),
                        attr: class_def.attr,
                        generic_count_requirement: class_def.get_generic_count_requirement().into(),
                        parent: class_def
                            .parent
                            .as_ref()
                            .map(|r| self.compile_type_reference(assembly, r))
                            .transpose()?,
                        method_table: class_def
                            .methods
                            .iter()
                            .enumerate()
                            .map(|(ind, x)| {
                                self.compile_method(
                                    assembly,
                                    &ty_info,
                                    &mut interface_implementations,
                                    &mut override_count,
                                    ind as u32,
                                    x,
                                )
                            })
                            .try_collect()?,
                        fields: class_def
                            .fields
                            .iter()
                            .map(|x| self.compile_field(assembly, x))
                            .try_collect()?,
                        sctor: None,
                        generic_bounds: None, // TODO

                        interfaces: interface_implementations
                            .iter()
                            .map(|(ty, map)| {
                                let mut last = 0u32;
                                Ok::<_, BuildError>(InterfaceImplementation {
                                    target: self.compile_type_reference(assembly, ty)?,
                                    map: map
                                        .iter()
                                        .map(|(k, v)| {
                                            assert_eq!(*k, last);
                                            last = *k;
                                            *v
                                        })
                                        .collect(),
                                })
                            })
                            .try_collect()?,
                    };
                    assembly
                        .type_defs
                        .push(binary::ty::TypeDef::Class(marshaled));
                }
                ast::ty::TypeDef::Struct(struct_def) => {
                    let ty_info = ty.to_abstract().into_ok();
                    let marshaled = binary::ty::StructDef {
                        name: assembly.add_string(struct_def.name.as_ref()),
                        attr: struct_def.attr,
                        generic_count_requirement: struct_def
                            .get_generic_count_requirement()
                            .into(),
                        method_table: struct_def
                            .methods
                            .iter()
                            .enumerate()
                            .map(|(ind, x)| {
                                self.compile_method(
                                    assembly,
                                    &ty_info,
                                    &mut HashMap::new(),
                                    &mut 0,
                                    ind as u32,
                                    x,
                                )
                            })
                            .try_collect()?,
                        fields: struct_def
                            .fields
                            .iter()
                            .map(|x| self.compile_field(assembly, x))
                            .try_collect()?,
                        sctor: None,
                        generic_bounds: None, // TODO
                    };
                    assembly
                        .type_defs
                        .push(binary::ty::TypeDef::Struct(marshaled));
                }
                ast::ty::TypeDef::Interface(interface_def) => {
                    let ty_info = ty.to_abstract().into_ok();
                    let marshaled = binary::ty::InterfaceDef {
                        name: assembly.add_string(interface_def.name.as_ref()),
                        attr: interface_def.attr,
                        generic_count_requirement: interface_def
                            .get_generic_count_requirement()
                            .into(),
                        required_interfaces: interface_def
                            .required_interfaces
                            .iter()
                            .map(|x| self.compile_type_reference(assembly, x))
                            .try_collect()?,
                        method_table: interface_def
                            .methods
                            .iter()
                            .enumerate()
                            .map(|(ind, x)| {
                                self.compile_method(
                                    assembly,
                                    &ty_info,
                                    &mut HashMap::new(),
                                    &mut 0,
                                    ind as u32,
                                    x,
                                )
                            })
                            .try_collect()?,
                        generic_bounds: None, // TODO
                    };
                    assembly
                        .type_defs
                        .push(binary::ty::TypeDef::Interface(marshaled));
                }
            }
        }

        Ok(())
    }

    fn resolve_type_reference(&self, r: &TypeReference) -> Option<TypeInformation> {
        let TypeReference::Common {
            assembly_name,
            name,
            index,
            generics,
        } = r
        else {
            return None;
        };

        let query_options = TypeQueryOptions::builder()
            .assembly_name(assembly_name.clone())
            .generic_count(generics.len() as u32)
            .type_name(name.clone())
            .try_decorate(false)
            .exact(true)
            .maybe_index(*index)
            .build();

        self.service.query(&query_options).pop()
    }

    fn compile_type_reference(
        &self,
        assembly: &mut AssemblyBuilder,
        r: &TypeReference,
    ) -> Result<TypeToken, BuildError> {
        match r {
            TypeReference::Common {
                assembly_name,
                name: _,
                index,
                generics,
            } => {
                let Some(id) = index.or_else(|| {
                    let ty = self.resolve_type_reference(r)?;
                    Some(ty.id)
                }) else {
                    return Err(BuildError::TypeNotFound(r.clone()));
                };

                if assembly_name == self.service.get_manifest().package().assembly_name()
                    && generics.is_empty()
                {
                    return Ok(TypeTokenBuilder::new_without_defaults()
                        .with_ty(TypeType::TypeDef)
                        .with_index(id)
                        .build());
                }

                let ty_ref = TypeRef {
                    assembly: assembly.add_string(assembly_name),
                    index: id,
                };
                let ty_ref_index = assembly.add_type_ref(ty_ref);
                let ty_ref_tk = TypeTokenBuilder::new_without_defaults()
                    .with_ty(TypeType::TypeRef)
                    .with_index(ty_ref_index)
                    .build();
                if generics.is_empty() {
                    return Ok(ty_ref_tk);
                }

                let generics = generics
                    .iter()
                    .map(|x| self.compile_type_reference(assembly, x))
                    .try_collect()?;

                let spec = TypeSpec {
                    ty: ty_ref_tk,
                    generics,
                };
                assembly.type_specs.push(spec);
                Ok(TypeTokenBuilder::new_without_defaults()
                    .with_ty(TypeType::TypeSpec)
                    .with_index((assembly.type_specs.len() - 1) as u32)
                    .build())
            }
            TypeReference::MethodGeneric(_, g_index) => {
                Ok(TypeTokenBuilder::new_without_defaults()
                    .with_ty(TypeType::MethodGeneric)
                    .with_index(*g_index)
                    .build())
            }
            TypeReference::TypeGeneric(_, g_index) => Ok(TypeTokenBuilder::new_without_defaults()
                .with_ty(TypeType::TypeGeneric)
                .with_index(*g_index)
                .build()),
        }
    }

    fn compile_field_reference(
        &self,
        _assembly: &mut AssemblyBuilder,
        container: Option<&TypeInformation>,
        is_static: bool,
        r: &ast::field::FieldReference,
    ) -> Result<u32, BuildError> {
        match r {
            ast::field::FieldReference::ByIndex(x) => Ok(*x),
            ast::field::FieldReference::ByName(rr) if let Some(container) = container => container
                .fields
                .iter()
                .find_map(|x| {
                    if (x.name == rr.var) && (x.attr.is_static() == is_static) {
                        Some(x.id)
                    } else {
                        None
                    }
                })
                .ok_or_else(|| BuildError::FieldNotFound(r.clone())),
            ast::field::FieldReference::ByName(rr) => {
                cfg_select! {
                    debug_assertions => {
                        eprintln!("Container not found for field `{}`", rr.var);
                    }
                    _ => { let _ = rr; }
                }
                Err(BuildError::FieldNotFound(r.clone()))
            }
        }
    }

    fn compile_method_reference(
        &self,
        assembly: &mut AssemblyBuilder,
        container: Option<&TypeInformation>,
        is_static: bool,
        r: &ast::method::MethodReference,
    ) -> Result<MethodToken, BuildError> {
        let index = match &r.index {
            ast::item::ItemRef::ByIndex(index) => *index,
            ast::item::ItemRef::ByName(rr) if let Some(container) = container => container
                .methods
                .iter()
                .find_map(|x| {
                    if (x.name == rr.var) && (x.is_static() == is_static) {
                        Some(x.id)
                    } else {
                        None
                    }
                })
                .ok_or_else(|| BuildError::MethodNotFound(r.clone()))?,
            ast::item::ItemRef::ByName(rr) => {
                cfg_select! {
                    debug_assertions => {
                        eprintln!("Container not found for field `{}`", rr.var);
                    }
                    _ => { let _ = rr; }
                }
                return Err(BuildError::MethodNotFound(r.clone()));
            }
        };

        if r.generics.is_empty() {
            Ok(MethodTokenBuilder::new_without_defaults()
                .with_ty(MethodType::Method)
                .with_index(index)
                .build())
        } else {
            let generics = r
                .generics
                .iter()
                .map(|r| self.compile_type_reference(assembly, r))
                .try_collect()?;

            let spec = MethodSpec { m: index, generics };

            assembly.method_specs.push(spec);

            Ok(MethodTokenBuilder::new_without_defaults()
                .with_ty(MethodType::MethodSpec)
                .with_index((assembly.method_specs.len() - 1) as u32)
                .build())
        }
    }

    fn compile_field(
        &self,
        assembly: &mut AssemblyBuilder,
        field: &ast::field::Field,
    ) -> Result<binary::ty::Field, BuildError> {
        Ok(binary::ty::Field {
            name: assembly.add_string(field.name.as_ref()),
            attr: field.attr,
            ty: self.compile_type_reference(assembly, &field.ty)?,
        })
    }

    fn compile_method(
        &self,
        assembly: &mut AssemblyBuilder,
        current_ty: &TypeInformation,
        interface_implementations: &mut HashMap<TypeReference, BTreeMap<u32, u32>>,
        override_count: &mut u32,
        current_index: u32,
        method: &ast::method::Method,
    ) -> Result<binary::ty::Method, BuildError> {
        fn map_variable(
            var: &Identifier,
            locals: &[Identifier],
        ) -> Result<RegisterAddr, BuildError> {
            Ok(RegisterAddr::new(
                locals
                    .iter()
                    .position(|x| x == var)
                    .ok_or(BuildError::VarNotFound(var.var.clone()))? as u64,
            ))
        }
        fn resolve_variable_type<'a>(
            var: &Identifier,
            this: &'a Compiler,
            method: &ast::method::Method,
        ) -> Option<TypeInformation> {
            resolve_variable_type_by_addr(map_variable(var, &method.locals).ok()?, this, method)
        }
        fn resolve_variable_type_by_addr<'a>(
            addr: RegisterAddr,
            this: &'a Compiler,
            method: &ast::method::Method,
        ) -> Option<TypeInformation> {
            let r = method.attr.local_variable_types().get(addr.get_usize())?;
            this.resolve_type_reference(r)
        }

        if method.attr.overrides().is_none() {
            let parent_len = match &current_ty.parent {
                Some(parent) => {
                    self.resolve_type_reference(parent).unwrap().methods.len() as u32 - 1
                }
                None => 0,
            };
            assert_eq!(
                method.index,
                current_index + parent_len - *override_count,
                "Method {} in {} has different indexes, expect {}, found {}",
                method.name,
                current_ty.name,
                method.index,
                current_index + parent_len - *override_count,
            );
        } else {
            *override_count += 1;
        }

        if let Some((ty, interface_method)) = method.r#impl.as_ref() {
            interface_implementations
                .entry(ty.clone())
                .or_default()
                .insert(
                    {
                        let r = self.compile_method_reference(
                            assembly,
                            self.resolve_type_reference(ty).as_ref(),
                            false,
                            interface_method,
                        )?;
                        std::assert_matches!(r.ty(), MethodType::Method);
                        r.index()
                    },
                    method.index,
                );
        }

        Ok(
            binary::ty::Method {
                name: assembly.add_string(method.name.as_ref()),
                attr: method
                    .attr
                    .clone()
                    .try_map_types(|x| self.compile_type_reference(assembly, &x))?,
                generic_count_requirement: method.get_generic_count_requirement().into(),
                args: method
                    .args
                    .iter()
                    .map(|orig| {
                        Ok::<_, BuildError>(binary::ty::Parameter {
                            ty: self.compile_type_reference(assembly, &orig.ty)?,
                            attr: orig.attr,
                        })
                    })
                    .try_collect()?,
                return_type: self.compile_type_reference(assembly, &method.return_type)?,
                call_convention: method.call_conv,
                generic_bounds: None, // TODO
                instructions: method
                    .statements
                    .iter()
                    .map(|x| -> Result<Instruction<_, _, _, _>, BuildError> {
                        match x {
                            ast::method::Statement::Load {
                                content: literal,
                                var,
                            } => {
                                let addr = RegisterAddr::new(
                                    method
                                        .locals
                                        .iter()
                                        .position(|x| x == var)
                                        .ok_or(BuildError::VarNotFound(var.var.clone()))?
                                        as u64,
                                );
                                let content = match literal {
                                    &ast::method::LoadableContent::U8(val) => LoadContent::U8(val),
                                    &ast::method::LoadableContent::U16(val) => {
                                        LoadContent::U16(val)
                                    }
                                    &ast::method::LoadableContent::U32(val) => {
                                        LoadContent::U32(val)
                                    }
                                    &ast::method::LoadableContent::U64(val) => {
                                        LoadContent::U64(val)
                                    }
                                    &ast::method::LoadableContent::I8(val) => LoadContent::I8(val),
                                    &ast::method::LoadableContent::I16(val) => {
                                        LoadContent::I16(val)
                                    }
                                    &ast::method::LoadableContent::I32(val) => {
                                        LoadContent::I32(val)
                                    }
                                    &ast::method::LoadableContent::I64(val) => {
                                        LoadContent::I64(val)
                                    }
                                    ast::method::LoadableContent::String(val) => {
                                        LoadContent::String(assembly.add_string(val))
                                    }
                                    ast::method::LoadableContent::Char(_val) => todo!(),
                                    ast::method::LoadableContent::ByteString(_val) => todo!(),
                                    ast::method::LoadableContent::Byte(_val) => todo!(),
                                    ast::method::LoadableContent::True => LoadContent::True,
                                    ast::method::LoadableContent::False => LoadContent::False,
                                    ast::method::LoadableContent::This => LoadContent::This,
                                    &ast::method::LoadableContent::Arg(arg) => {
                                        LoadContent::Arg(arg)
                                    }
                                    ast::method::LoadableContent::Static { ty, field } => {
                                        let ty_token = self.compile_type_reference(assembly, ty)?;
                                        let field = self.compile_field_reference(
                                            assembly,
                                            self.resolve_type_reference(ty).as_ref(),
                                            true,
                                            field,
                                        )?;
                                        LoadContent::Static {
                                            ty: ty_token,
                                            field,
                                        }
                                    }
                                    ast::method::LoadableContent::Field { container, field } => {
                                        let container = map_variable(container, &method.locals)?;
                                        let field = self.compile_field_reference(
                                            assembly,
                                            resolve_variable_type_by_addr(container, self, method)
                                                .as_ref(),
                                            false,
                                            field,
                                        )?;
                                        LoadContent::Field { container, field }
                                    }
                                    ast::method::LoadableContent::Size(ty) => {
                                        let ty = self.compile_type_reference(assembly, ty)?;
                                        LoadContent::TypeValueSize(ty)
                                    }
                                };

                                Ok(Instruction::Load(Instruction_Load { addr, content }))
                            }

                            ast::method::Statement::ReadPointerTo {
                                ptr,
                                size,
                                destination,
                            } => {
                                let ptr = map_variable(ptr, &method.locals)?;
                                let size = map_variable(size, &method.locals)?;
                                let destination = map_variable(destination, &method.locals)?;
                                Ok(Instruction::ReadPointerTo(CommonReadPointerTo {
                                    ptr,
                                    size,
                                    destination,
                                }))
                            }
                            ast::method::Statement::WritePointer { source, size, ptr } => {
                                let source = map_variable(source, &method.locals)?;
                                let size = map_variable(size, &method.locals)?;
                                let ptr = map_variable(ptr, &method.locals)?;
                                Ok(Instruction::WritePointer(CommonWritePointer {
                                    source,
                                    size,
                                    ptr,
                                }))
                            }

                            ast::method::Statement::Check {
                                kind,
                                to_check,
                                result,
                            } => {
                                let output = map_variable(result, &method.locals)?;
                                let to_check = map_variable(to_check, &method.locals)?;
                                let content = match kind {
                                    ast::method::CheckKind::AllZero(_kw) => {
                                        ToCheckContent::IsAllZero(to_check)
                                    }
                                };
                                Ok(Instruction::Check(Instruction_CommonCheck {
                                    output,
                                    content,
                                }))
                            }

                            ast::method::Statement::NewObject {
                                ty,
                                ctor,
                                args,
                                result,
                            } => {
                                let ty_token = self.compile_type_reference(assembly, ty)?;
                                let ctor_name = self.compile_method_reference(
                                    assembly,
                                    self.resolve_type_reference(ty).as_ref(),
                                    false,
                                    ctor,
                                )?;
                                let args = args
                                    .iter()
                                    .map(|x| map_variable(x, &method.locals))
                                    .try_collect()?;
                                let output = map_variable(result, &method.locals)?;
                                Ok(Instruction::New(Instruction_New::NewObject {
                                    ty: ty_token,
                                    ctor_name,
                                    args,
                                    output,
                                }))
                            }
                            ast::method::Statement::NewArray {
                                element_ty,
                                len,
                                result,
                            } => {
                                let element_type =
                                    self.compile_type_reference(assembly, element_ty)?;
                                let output = map_variable(result, &method.locals)?;
                                match len {
                                    ArrayLen::Dynamic(len) => {
                                        let len_addr = map_variable(len, &method.locals)?;
                                        Ok(Instruction::New(Instruction_New::NewDynamicArray {
                                            element_type,
                                            len_addr,
                                            output,
                                        }))
                                    }
                                    &ArrayLen::Static(len) => {
                                        Ok(Instruction::New(Instruction_New::NewArray {
                                            element_type,
                                            len,
                                            output,
                                        }))
                                    }
                                }
                            }

                            ast::method::Statement::InstanceCall {
                                val,
                                method: method_ref,
                                args,
                                result,
                            } => {
                                let val_addr = map_variable(val, &method.locals)?;
                                let method_ref = self.compile_method_reference(
                                    assembly,
                                    resolve_variable_type(val, self, method).as_ref(),
                                    false,
                                    method_ref,
                                )?;
                                let args = args
                                    .iter()
                                    .map(|x| map_variable(x, &method.locals))
                                    .try_collect()?;
                                let ret_at = map_variable(result, &method.locals)?;
                                Ok(Instruction::Call(Instruction_Call::InstanceCall {
                                    val: val_addr,
                                    method: method_ref,
                                    args,
                                    ret_at,
                                }))
                            }
                            ast::method::Statement::StaticCall {
                                ty,
                                method: method_ref,
                                args,
                                result,
                            } => {
                                let ty_token = self.compile_type_reference(assembly, ty)?;
                                let method_ref = self.compile_method_reference(
                                    assembly,
                                    self.resolve_type_reference(ty).as_ref(),
                                    true,
                                    method_ref,
                                )?;
                                let args = args
                                    .iter()
                                    .map(|x| map_variable(x, &method.locals))
                                    .try_collect()?;
                                let ret_at = map_variable(result, &method.locals)?;
                                Ok(Instruction::Call(Instruction_Call::StaticCall {
                                    ty: ty_token,
                                    method: method_ref,
                                    args,
                                    ret_at,
                                }))
                            }
                            ast::method::Statement::InterfaceCall {
                                val,
                                interface,
                                method: method_ref,
                                args,
                                result,
                            } => {
                                let val_addr = map_variable(val, &method.locals)?;
                                let interface = self.compile_type_reference(assembly, interface)?;
                                let method_ref = self.compile_method_reference(
                                    assembly,
                                    resolve_variable_type(val, self, method).as_ref(),
                                    false,
                                    method_ref,
                                )?;
                                let args = args
                                    .iter()
                                    .map(|x| map_variable(x, &method.locals))
                                    .try_collect()?;
                                let ret_at = map_variable(result, &method.locals)?;
                                Ok(Instruction::Call(Instruction_Call::InterfaceCall {
                                    interface,
                                    val: val_addr,
                                    method: method_ref,
                                    args,
                                    ret_at,
                                }))
                            }
                            ast::method::Statement::NonPurusCall {
                                config,
                                f_pointer,
                                args,
                                result,
                            } => {
                                let f_pointer = map_variable(f_pointer, &method.locals)?;
                                let config = map_variable(config, &method.locals)?;
                                let args = args
                                    .iter()
                                    .map(|x| map_variable(x, &method.locals))
                                    .try_collect()?;
                                let ret_at = map_variable(result, &method.locals)?;
                                Ok(Instruction::Call(Instruction_Call::DynamicNonPurusCall {
                                    f_pointer,
                                    config,
                                    args,
                                    ret_at,
                                }))
                            }

                            ast::method::Statement::SetField {
                                val,
                                container,
                                field,
                            } => {
                                let val = map_variable(val, &method.locals)?;
                                let container = map_variable(container, &method.locals)?;
                                let field = self.compile_field_reference(
                                    assembly,
                                    Some(current_ty),
                                    false,
                                    field,
                                )?;
                                Ok(Instruction::Set(Instruction_Set::Common {
                                    val,
                                    container,
                                    field,
                                }))
                            }
                            ast::method::Statement::SetThisField { val, field } => {
                                let val = map_variable(val, &method.locals)?;
                                let field = self.compile_field_reference(
                                    assembly,
                                    Some(current_ty),
                                    false,
                                    field,
                                )?;
                                Ok(Instruction::Set(Instruction_Set::This { val, field }))
                            }
                            ast::method::Statement::SetStaticField { val, ty, field } => {
                                let val = map_variable(val, &method.locals)?;
                                let ty_token = self.compile_type_reference(assembly, ty)?;
                                let field = self.compile_field_reference(
                                    assembly,
                                    self.resolve_type_reference(ty).as_ref(),
                                    true,
                                    field,
                                )?;
                                Ok(Instruction::Set(Instruction_Set::Static {
                                    val,
                                    ty: ty_token,
                                    field,
                                }))
                            }
                            ast::method::Statement::Throw { val } => {
                                let exception_addr = map_variable(val, &method.locals)?;
                                Ok(Instruction::Throw { exception_addr })
                            }
                            ast::method::Statement::ReturnVal { val } => {
                                let register_addr = map_variable(val, &method.locals)?;
                                Ok(Instruction::ReturnVal { register_addr })
                            }
                            ast::method::Statement::Jump { condition, ty, val } => {
                                let target = JumpTargetBuilder::new_without_defaults()
                                .with_ty(match ty {
                                    ast::method::JumpTargetType::Absolute(_kw) => {
                                        pura_lingua::global::instruction::JumpTargetType::Absolute
                                    }
                                    ast::method::JumpTargetType::Forward(_kw) => {
                                        pura_lingua::global::instruction::JumpTargetType::Forward
                                    }
                                    ast::method::JumpTargetType::Backward(_kw) => {
                                        pura_lingua::global::instruction::JumpTargetType::Backward
                                    }
                                })
                                .with_val(*val)
                                .build();
                                let condition = match condition {
                                    ast::method::JumpCondition::Unconditional => {
                                        JumpCondition::Unconditional
                                    }
                                    ast::method::JumpCondition::IfTrue(_kw_if, val) => {
                                        let register_addr = map_variable(val, &method.locals)?;
                                        JumpCondition::If(register_addr)
                                    }
                                    ast::method::JumpCondition::CheckSuccess(
                                        _kw_success,
                                        check_kind,
                                        to_check,
                                    ) => match check_kind {
                                        ast::method::CheckKind::AllZero(_kw) => {
                                            let to_check = map_variable(to_check, &method.locals)?;
                                            JumpCondition::IfCheckSucceeds(
                                                ToCheckContent::IsAllZero(to_check),
                                            )
                                        }
                                    },
                                    ast::method::JumpCondition::CheckFailure(
                                        _kw_failure,
                                        check_kind,
                                        to_check,
                                    ) => match check_kind {
                                        ast::method::CheckKind::AllZero(_kw) => {
                                            let to_check = map_variable(to_check, &method.locals)?;
                                            JumpCondition::IfCheckFails(ToCheckContent::IsAllZero(
                                                to_check,
                                            ))
                                        }
                                    },
                                };
                                Ok(Instruction::Jump(Instruction_Jump { target, condition }))
                            }
                            ast::method::Statement::Nop => Ok(Instruction::Nop),
                        }
                    })
                    .try_collect()?,
                exception_table:
                    method
                        .exception_table
                        .iter()
                        .map(|entry| {
                            #[derive(Copy)]
                            #[derive_const(Clone)]
                            struct ResolveRange {
                                try_begin: u64,
                            }
                            impl const
                                FnOnce<((ast::method::ExceptionLoc, ast::method::ExceptionLoc),)>
                                for ResolveRange
                            {
                                type Output = Range<u64>;
                                #[inline(always)]
                                extern "rust-call" fn call_once(
                                    self,
                                    ((start, end),): ((
                                        ast::method::ExceptionLoc,
                                        ast::method::ExceptionLoc,
                                    ),),
                                ) -> Self::Output {
                                    Range {
                                        start: start.resolve(self.try_begin),
                                        end: end.resolve(self.try_begin),
                                    }
                                }
                            }
                            let resolve_range = ResolveRange {
                                try_begin: entry.try_begin,
                            };
                            Ok(binary::ty::ExceptionTableEntry {
                                range: Range {
                                    start: entry.try_begin,
                                    end: entry.try_end.resolve(entry.try_begin),
                                },
                                exception_type: self
                                    .compile_type_reference(assembly, &entry.exception_type)?,
                                filter: entry
                                    .filter
                                    .as_ref()
                                    .map(|(ty, method)| {
                                        self.compile_type_reference(assembly, ty).and_then(
                                            |ty_token| {
                                                self.compile_method_reference(
                                                    assembly,
                                                    self.resolve_type_reference(ty).as_ref(),
                                                    true,
                                                    method,
                                                )
                                                .map(|method| (ty_token, method))
                                            },
                                        )
                                    })
                                    .transpose()?,
                                catch: resolve_range(entry.catch),
                                finally: entry.finally.map(resolve_range),
                                fault: entry.fault.map(resolve_range),
                            })
                        })
                        .try_collect()?,
            },
        )
    }
}
