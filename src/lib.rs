#![allow(non_snake_case)]
#![feature(unwrap_infallible)]
#![feature(const_clone)]
#![feature(derive_const)]
#![feature(const_trait_impl)]
#![feature(unboxed_closures)]
#![feature(fn_traits)]
#![feature(lock_value_accessors)]
#![feature(iterator_try_collect)]

use std::{
    ops::RangeBounds,
    sync::{Mutex, RwLock},
};

use compiler_base::{
    BuildOptions, CompilerPtr, IBuild, ICompiler, ITypeQuery, TypeQueryOptions,
    abstract_info::{IToAbstract, type_information::TypeInformation},
    boxed::BoxedCompiler,
    span::FilePath,
};
use compiler_service::{ICompilerService, reference::CompilerServiceRef};
use pura_lingua::{binary::assembly::AssemblyBuilder, global};

#[cfg(test)]
mod tests;

pub struct Compiler {
    pub(crate) service: CompilerServiceRef,
    pub(crate) file: RwLock<ast::File>,
    pub(crate) last_index: Mutex<Option<u32>>,
}

impl Compiler {
    pub fn new(service: CompilerServiceRef) -> Self {
        Self {
            service,
            file: RwLock::new(ast::File::new()),
            last_index: Mutex::new(None),
        }
    }
}

impl ITypeQuery for Compiler {
    fn query(&self, options: &TypeQueryOptions) -> Vec<TypeInformation> {
        if options.lang_id.as_ref().is_some_and(|x| x != COMPILER_ID) {
            return Vec::new();
        }

        if options
            .assembly_name
            .as_ref()
            .is_some_and(|x| x != self.service.get_manifest().package().assembly_name())
        {
            return Vec::new();
        }

        let file = self.file.read().unwrap();
        let mut iter: Box<dyn Iterator<Item = TypeInformation>> = Box::new(
            file.types
                .iter()
                .map(IToAbstract::to_abstract)
                .map(Result::into_ok),
        );

        if let Some(index) = options.index {
            return if options.exact {
                Vec::from_iter(iter.find(|x| x.id == index).into_iter())
            } else {
                iter.filter(|x| x.id == index).collect()
            };
        }

        if let Some(generic_count) = options.generic_count {
            iter = Box::new(iter.filter(move |x| x.generic_count.contains(&generic_count)));
        }

        if let Some(type_name) = options.type_name.as_ref() {
            iter = Box::new(iter.filter(|x| {
                x.name.eq(type_name)
                    || (options.try_decorate && {
                        let mut raw_name = type_name.clone();
                        x.generic_count.decorate(&mut raw_name);
                        x.name == raw_name
                    })
            }));
        }

        if let Some(required_interface) = options.implemented_interface.as_ref() {
            iter = Box::new(iter.filter(|x| x.implemented_interfaces.contains(required_interface)));
        }

        if options.exact {
            iter = Box::new(iter.take(1));
        }

        iter.collect()
    }
}

mod build_impl;

impl IBuild for Compiler {
    fn build(&self, assembly: &mut AssemblyBuilder, options: &BuildOptions) -> global::Result<()> {
        self.build_impl(assembly, options)
    }
}

impl ICompiler for Compiler {
    fn set_last_index(&self, index: u32) {
        self.last_index.set(Some(index)).unwrap();
    }
    fn add_file(&self, file_path: &FilePath) -> global::Result<()> {
        let last_index = self.last_index.get_cloned().unwrap().unwrap();

        let content = String::from_utf8(self.service.read_path(file_path)?)?;

        let file = ast::parse(content, last_index)?;

        let mut out_file = self.file.write().unwrap();
        out_file.merge(file);
        out_file.sort();

        Ok(())
    }

    /// Return last index
    fn finish_load(&self) -> global::Result<u32> {
        let file = self.file.read().unwrap();
        Ok(file
            .types
            .iter()
            .map(ast::ty::TypeDef::index)
            .max()
            .unwrap())
    }
}

#[unsafe(no_mangle)]
pub static SOURCE_SUFFIX: &str = "plir";

#[unsafe(no_mangle)]
pub static COMPILER_ID: &str = include_str!("../__LANG_ID");

#[unsafe(no_mangle)]
pub extern "C" fn NewCompiler(service: CompilerServiceRef) -> CompilerPtr {
    BoxedCompiler::new(Compiler::new(service)).into_raw()
}
