remu_macro::mod_flat!(instruction_fetch, inst_enum, patterns, instruction_decoder, arithmetic_logic, address_generation, load_store, write_back, executer, trap);

remu_macro::mod_pub!(frontend, backend);
