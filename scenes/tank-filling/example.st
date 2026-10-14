PROGRAM Main

VAR_INPUT
    LevelLow : BOOL;
    LevelHigh : BOOL;
    LevelPercent : REAL;
END_VAR

VAR_OUTPUT
    InletValve : BOOL;
    DrainPump : BOOL;
END_VAR

InletValve := NOT LevelHigh;
DrainPump := FALSE;

END_PROGRAM
