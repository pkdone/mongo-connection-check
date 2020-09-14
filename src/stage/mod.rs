const STAGE0: usize = 0;
pub const STAGE1: usize = 1;
pub const STAGE2: usize = 2;
pub const STAGE3: usize = 3;
pub const STAGE4: usize = 4;
pub const STAGE5: usize = 5;
pub const STAGE6: usize = 6;
pub const STAGE7: usize = 7;
const TOTAL_STAGES: usize = STAGE7 + 1;


pub struct Stage {
    pub index: usize,
    pub name: &'static str,
    pub desc: &'static str,
}


pub const STAGES: [Stage; TOTAL_STAGES] = [
    Stage {
        index: STAGE0,
        name: "<setup>",
        desc: "<setup>",
    },
    Stage {
        index: STAGE1,
        name: "URL-CHECK",
        desc: "Confirm URL contains seed list of target server(s) or service name",
    },
    Stage {
        index: STAGE2,
        name: "MEMBERS-CHECK",
        desc: "Determine list of individual servers (look up DNS SRV service if defined)",
    },
    Stage {
        index: STAGE3,
        name: "DNS-IP-CHECK",
        desc: "Determine the IP addresses of each individual server, via DNS",
    },
    Stage {
        index: STAGE4,
        name: "SOCKET-CHECK",
        desc: "Confirm TCP socket can be established to one or more target servers",
    },
    Stage {
        index: STAGE5,
        name: "DRIVER-CHECK",
        desc: "Confirm driver can validate the URL (including SRV resolution if required)",
    },
    Stage {
        index: STAGE6,
        name: "DBPING-CHECK",
        desc: "Confirm driver can connect to deployment using 'dbping'",
    },
    Stage {
        index: STAGE7,
        name: "HEALTH-CHECK",
        desc: "Retrieve running deployment's member composition including which is primary",
    },
];


#[derive(Debug)]
pub enum StageState {
    NotApplicable,
    NotTested,
    Passed,
    Failed,
}


pub struct StageStatus {
    pub index: usize,
    pub state: StageState,
    pub advice: Vec<String>,
}


// Initialise the structure for tracking stage checks to all be 'NotTested', initially
// 
impl StageStatus {
    pub fn new_set()
                   -> [StageStatus; TOTAL_STAGES] {
        [
            StageStatus {
                index: STAGE0,
                state: StageState::NotApplicable,
                advice: vec![],
            },
            StageStatus {
                index: STAGE1,
                state: StageState::NotTested,
                advice: vec![],
            },
            StageStatus {
                index: STAGE2,
                state: StageState::NotTested,
                advice: vec![],
            },
            StageStatus {
                index: STAGE3,
                state: StageState::NotTested,
                advice: vec![],
            },
            StageStatus {
                index: STAGE4,
                state: StageState::NotTested,
                advice: vec![],
            },
            StageStatus {
                index: STAGE5,
                state: StageState::NotTested,
                advice: vec![],
            },
            StageStatus {
                index: STAGE6,
                state: StageState::NotTested,
                advice: vec![],
            },
            StageStatus {
                index: STAGE7,
                state: StageState::NotTested,
                advice: vec![],
            },
        ]
    }
}   

