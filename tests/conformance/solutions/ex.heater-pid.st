PROGRAM Main
VAR_INPUT
    AutoMode : BOOL; (* Selector S1 in position AUTO. *)
    TempRaw : INT; (* Temperature transmitter TT-1: 0..10000 = 0..150 degC. *)
END_VAR
VAR_OUTPUT
    HeaterRaw : INT; (* Heater power controller: 0..10000 = 0..100 %. *)
    HeaterLamp : BOOL; (* Lamp H1: the heater is on. *)
    AlarmLamp : BOOL; (* Lamp H2: overtemperature. *)
    Temperature : REAL; (* Vessel temperature in degC. *)
END_VAR
VAR
    Setpoint : REAL := 60.0; (* degC *)
    Kp : REAL := 4.0; (* proportional gain, % per degC *)
    Ti : TIME := T#20s; (* integral time *)
    Td : TIME := T#0s; (* derivative time *)
    PowerMin : REAL := 0.0; (* output limits, % *)
    PowerMax : REAL := 100.0;
    ManualPower : REAL := 20.0; (* heater power in MAN, % *)
    Power : REAL; (* controller output, % *)
    Loop : PID;
END_VAR

Temperature := INT_TO_REAL(TempRaw) * 0.015;

Loop(SP := Setpoint, PV := Temperature, KP := Kp, TI := Ti, TD := Td,
     MV_MIN := PowerMin, MV_MAX := PowerMax,
     MANUAL := NOT AutoMode, MV_MAN := ManualPower, MV => Power);

HeaterRaw := REAL_TO_INT(Power * 100.0);
HeaterLamp := Power > 0.5;
AlarmLamp := Temperature > 90.0;
END_PROGRAM
