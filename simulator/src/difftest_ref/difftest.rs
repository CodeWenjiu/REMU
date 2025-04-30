// use option_parser::OptionParser;
// use state::States;

// use crate::SimulatorCallback;

// use super::AnyDifftestRef;

// pub struct DifftestRef {
//     pub reference: AnyDifftestRef,
//     pub states: States,
//     pub dut_states: States,
// }

// impl DifftestRef {
//     pub fn new(
//         option: &OptionParser,
//         states_dut: States,
//         states_ref: States,
//     ) -> Self {
//         // Create a minimal callback for the reference simulator, may be useful in future
//         let ref_callback = SimulatorCallback::new(
//             Box::new(|_: u32, _: u32| Ok(())),
//             Box::new(|| {}),
//             Box::new(|_: u32, _: u32| {}),
//             Box::new(|_: bool| {}),
//         );

//         let reference = AnyDifftestRef::try_from((option, states_ref.clone(), ref_callback));

//         Self {
//             reference: reference.unwrap(),
//             states: states_dut,
//             dut_states: states_ref,
//         }
//     }
// }

