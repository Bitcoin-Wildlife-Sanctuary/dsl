# dsl

This repository contains a domain-specific language (DSL)-like toolchain that converts a structural program into the 
corresponding Bitcoin script. Unlike [sCrypt](https://scrypt.io/), this was not designed for general users, but for 
the purpose of addressing a few challenges that we encounter in the new algorithm for M31.

These challenges include:
- difficulty to keep track of the relative position of the lookup table and other state data
- difficulty to do bookkeeping of the hints for unstructured programs with mixed function calls
- difficulty to read, debug, and maintain manually written copy (`OP_PICK`) and move (`OP_ROLL`) of stack operations

This DSL is still under development, and new functionalities will be added gradually as we try it on the new M31 algorithm. 
- more interfaces for the trace generator
- more documentation
- more examples for integration and tests

This implementation is inspired by the methods in sCrypt. Xiaohui Liu from sCrypto has provided some advice and feedback 
to this work.