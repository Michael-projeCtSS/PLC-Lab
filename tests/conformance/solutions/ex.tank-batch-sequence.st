PROGRAM Main
VAR_INPUT
    StartPB : BOOL; (* Start button S1 (normally open). *)
    StopPB : BOOL; (* Stop button S2 (normally closed: TRUE while not pressed). *)
    LevelLow : BOOL; (* TRUE while the level is below the low switch (LSL-104 contact closes when dry). *)
    LevelHigh : BOOL; (* TRUE while the level is at or above the high switch (LSH-103 contact closes when wet). *)
    InletOpen : BOOL; (* Inlet valve fully-open limit switch. *)
    LevelRaw : INT; (* Raw analog value of LT-105 (scaling depends on the selected platform profile). *)
END_VAR
VAR_OUTPUT
    InletValve : BOOL; (* Inlet valve solenoid XV-101: TRUE opens the valve. *)
    DrainPump : BOOL; (* Drain pump P-102 contactor. *)
END_VAR
VAR
    Step : INT;
END_VAR

IF NOT StopPB THEN
    Step := 0;
END_IF;
CASE Step OF
    0: IF StartPB AND StopPB THEN Step := 1; END_IF;
    1: IF LevelHigh THEN Step := 2; END_IF;
    2: IF LevelLow THEN Step := 0; END_IF;
END_CASE;
InletValve := Step = 1;
DrainPump := Step = 2 AND NOT LevelLow;

END_PROGRAM
