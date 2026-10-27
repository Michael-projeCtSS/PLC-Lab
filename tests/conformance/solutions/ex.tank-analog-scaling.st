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
    LevelPercent : REAL; (* Level 0..100 % computed from LevelRaw *)
    HighAlarm : BOOL; (* Level above 85 % *)
END_VAR
VAR
    Filling : BOOL;
END_VAR

LevelPercent := INT_TO_REAL(LevelRaw) / 100.0;
HighAlarm := LevelPercent > 85.0;
Filling := (StartPB OR Filling) AND StopPB AND LevelPercent < 60.0 AND NOT HighAlarm;
InletValve := Filling;
DrainPump := FALSE;

END_PROGRAM
