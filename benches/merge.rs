#![feature(test)]

extern crate test;
extern crate indexing;

use test::Bencher;

use indexing::indices;
use std::cmp;
use std::mem::swap;
