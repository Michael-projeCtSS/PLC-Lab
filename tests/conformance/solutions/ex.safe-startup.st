PROGRAM Main
VAR_INPUT
    StartPB : BOOL; (* Start button S1 (normally open). *)
    StopPB : BOOL; (* Stop button S2 (normally closed: TRUE while not pressed). *)
    LevelLow : BOOL; (* TRUE while the level is below the low switch (LSL-104 contact closes when dry). *)
    LevelHigh : BOOL; (* TRUE while the level is at or above the high switch (LSH-103 contact closes when wet). *)
    InletOpen : BOOL; (* Inlet valve fully-open limit switch. *)
    LevelRaw : INT; (* Raw analog value of LT-105. *)
END_VAR
VAR_OUTPUT
    InletValve : BOOL; (* Inlet valve solenoid XV-101: TRUE opens the valve. *)
    DrainPump : BOOL; (* Drain pump P-102 contactor. *)
END_VAR
VAR RETAIN
    Filling : BOOL; (* Fill request. *)
    Batches : INT; (* Batches filled in this shift. *)
END_VAR
VAR
    Edge : R_TRIG;
END_VAR

(* Start fills the tank to the high level switch and counts the batch. *)
(* After every start the inlet stays closed until the operator presses Start again. *)
IF FIRST_SCAN THEN
    Filling := FALSE;
END_IF;
Edge(CLK := StartPB);
IF Edge.Q AND NOT LevelHigh THEN
    Filling := TRUE;
END_IF;
IF Filling AND LevelHigh THEN
    Filling := FALSE;
    Batches := Batches + 1;
END_IF;
IF NOT StopPB THEN
    Filling := FALSE;
END_IF;
InletValve := Filling;
DrainPump := FALSE;

END_PROGRAM
