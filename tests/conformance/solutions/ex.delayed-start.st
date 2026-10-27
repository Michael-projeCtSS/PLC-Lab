PROGRAM Main
VAR_INPUT
    StartPB : BOOL; (* Start button S1, normally open: TRUE while pressed. *)
    StopPB : BOOL; (* Stop button S2, normally closed: TRUE while NOT pressed (wire-break safe). *)
    MotorFeedback : BOOL; (* Contactor auxiliary contact: TRUE when the contactor has pulled in. *)
    OverloadOK : BOOL; (* Overload relay contact, normally closed: TRUE while the motor is not overloaded. *)
END_VAR
VAR_OUTPUT
    MotorContactor : BOOL; (* Contactor coil K1: TRUE energizes the motor. *)
    RunLamp : BOOL; (* Green run lamp H1. *)
END_VAR
VAR
    Request : BOOL;
    Delay : TON;
END_VAR

Request := (StartPB OR Request) AND StopPB AND OverloadOK;
Delay(IN := Request, PT := T#3s);
MotorContactor := Delay.Q;
RunLamp := Request;

END_PROGRAM
