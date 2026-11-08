PROGRAM Main
VAR_INPUT
    StartPB : BOOL;
    StopPB : BOOL;
    HoldPB : BOOL;
    LevelLow : BOOL;
    LevelHigh : BOOL;
    InletOpen : BOOL;
END_VAR
VAR_OUTPUT
    InletValve : BOOL;
    DrainPump : BOOL;
END_VAR
VAR
    Step : INT;
    Held : BOOL;
END_VAR

(* Stop aborts the batch from every step. *)
IF NOT StopPB THEN
    Step := 0;
    Held := FALSE;
END_IF;

(* HOLD keeps a running step and stops the plant; Start resumes the same step. *)
IF Step >= 1 AND Step <= 4 THEN
    IF HoldPB THEN
        Held := TRUE;
    ELSIF Held AND StartPB THEN
        Held := FALSE;
    END_IF;
ELSE
    Held := FALSE;
END_IF;

IF NOT Held THEN
    CASE Step OF
        0:
            IF StartPB AND StopPB AND NOT HoldPB AND NOT LevelHigh AND NOT InletOpen THEN
                Step := 1;
            END_IF;
        1:
            IF InletOpen THEN
                Step := 2;
            END_IF;
        2:
            IF LevelHigh THEN
                Step := 3;
            END_IF;
        3:
            IF NOT InletOpen AND NOT LevelLow THEN
                Step := 4;
            END_IF;
        4:
            IF LevelLow THEN
                Step := 5;
            END_IF;
        5:
            IF StartPB THEN
                Step := 0;
            END_IF;
    END_CASE;
END_IF;

InletValve := (Step = 1 OR Step = 2) AND NOT Held;
DrainPump := Step = 4 AND NOT Held;
END_PROGRAM
