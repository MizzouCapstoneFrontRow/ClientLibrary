#include <wiringPi.h>
#include <stdio.h>
#include <stdlib.h>
#include <stdbool.h>
#include "client.h"
#include <unistd.h>

// PI PIN DEFINITIONS
#define LIFT_FORWARD 1
#define LIFT_BACKWARD 2
#define LIFT_PWM 3

#define Left_WHEEL_FORWARD 4
#define LEFT_WHEEL_BACKWARD 5
#define LEFT_WHEEL_PWM 6

#define RIGHT_WHEEL_FORWARD 7
#define RIGHT_WHEEL_BACKWARD 8
#define RIGHT_WHEEL_PWM 9

#define LIFT_TOP_ENDSTOP 10
#define LIFT_BOTTOM_ENDSTOP 11

#define LIFT_ENCODER_PLUS 12
#define LIFT_ENCODER_MINUS 13 

#define RANGE 1023 // range is 0 - 1023 // PWM expects a number between 0 and 1023

void left_wheel_callback(const void * const*const parameters, void *const*const returns) {
    (void) returns;

    printf("Hello from left_wheel callback, %s!\n", *((const char**)parameters[0]));
    double x = *(const double*) parameters[0];

    if (x > 0) { // going forward on leftwheel
        digitalWrite(Left_WHEEL_FORWARD, HIGH);
        digitalWrite(LEFT_WHEEL_BACKWARD, LOW);
        pwmWrite(LEFT_WHEEL_PWM, (int) x * RANGE); 
    }
    else {
        digitalWrite(Left_WHEEL_FORWARD, LOW);
        digitalWrite(LEFT_WHEEL_BACKWARD, HIGH);
        pwmWrite(LEFT_WHEEL_PWM, (int) x * (-RANGE));
    }
}
const char * left_wheel_parameters[][2] = {
    {"x", "double"}, // this is the speed at which to go
    {NULL, NULL},
};
const char * left_wheel_returns[][2] = {
    {NULL, NULL},
};

void right_wheel_callback(const void * const*const parameters, void *const*const returns) {
    (void) returns;
    double x = *(const double*) parameters[0];
    printf("Hello from right_wheel callback, %s!\n", *((const char**)parameters[0]));
    
    if (x > 0) { // going forward on right wheel
        digitalWrite(RIGHT_WHEEL_FORWARD, HIGH);
        digitalWrite(RIGHT_WHEEL_BACKWARD, LOW);
        pwmWrite(RIGHT_WHEEL_PWM, (int) x * RANGE); 
    }
    else {
        digitalWrite(RIGHT_WHEEL_BACKWARD, LOW);
        digitalWrite(LEFT_WHEEL_BACKWARD, HIGH);
        pwmWrite(RIGHT_WHEEL_PWM, (int) x * (-RANGE));
    }
}

const char * right_wheel_parameters[][2] = {
    {"x", "double"},
    {NULL, NULL},
};
const char * right_wheel_returns[][2] = {
    {NULL, NULL},
};

void front_back_axis(const double value) {
    printf("Forward-Backward axis got %lf.\n", value);

    if (value > 0) { // going forward
        digitalWrite(RIGHT_WHEEL_FORWARD, HIGH);
        digitalWrite(Left_WHEEL_FORWARD, HIGH);
        digitalWrite(RIGHT_WHEEL_BACKWARD, LOW);
        digitalWrite(RIGHT_WHEEL_BACKWARD, LOW);
        pwmWrite(LEFT_WHEEL_PWM, (int) (value * RANGE));
        pwmWrite(RIGHT_WHEEL_PWM, (int) (value * RANGE));
    }
    else { // going backwards
        digitalWrite(RIGHT_WHEEL_FORWARD, LOW);
        digitalWrite(Left_WHEEL_FORWARD, LOW);
        digitalWrite(RIGHT_WHEEL_BACKWARD, HIGH);
        digitalWrite(RIGHT_WHEEL_BACKWARD, HIGH);
        pwmWrite(LEFT_WHEEL_PWM, (int) (value* (-1) * RANGE));
        pwmWrite(RIGHT_WHEEL_PWM, (int) (value * (-1) * RANGE));
    }
}

void lift_axis (const double value) {
    printf("LIFT Axis got %lf.\n", value);
       if (value > 0) { // going UP/FORWARD
        digitalWrite(LIFT_FORWARD, HIGH);
        digitalWrite(LIFT_BACKWARD, LOW);
        pwmWrite(LIFT_PWM, (int) (value * RANGE));
    }
    else { // Going down/backward
        digitalWrite(LIFT_FORWARD, LOW);
        digitalWrite(LIFT_BACKWARD, HIGH);
        pwmWrite(LIFT_PWM, (int) (value * (-1) * RANGE));
    }
}


int main() {
    ClientHandle handle = InitializeLibrary();
    enum ErrorCode result;


    // Setup WiringPI and pinmodes
    wiringPiSetup();
    pinMode(LIFT_FORWARD, OUTPUT);
    pinMode(LIFT_BACKWARD, OUTPUT);
    pinMode(LIFT_PWM, PWM_OUTPUT);

    pinMode(Left_WHEEL_FORWARD, OUTPUT);
    pinMode(LEFT_WHEEL_BACKWARD, OUTPUT);
    pinMode(LEFT_WHEEL_PWM, PWM_OUTPUT);

    pinMode(RIGHT_WHEEL_FORWARD, OUTPUT);
    pinMode(RIGHT_WHEEL_BACKWARD, OUTPUT);
    pinMode(RIGHT_WHEEL_PWM, PWM_OUTPUT);

    pinMode(LIFT_TOP_ENDSTOP, INPUT);
    pinMode(LIFT_BOTTOM_ENDSTOP, INPUT);
    pinMode(LIFT_ENCODER_PLUS, INPUT);
    pinMode(LIFT_ENCODER_MINUS, INPUT);


    puts("setting name");
    result = SetName(handle, "Example_pi");
    printf("result: %d\n", (int)result);

    printf("registering \"left_wheel\" function");
    result = RegisterFunction(handle, "left_wheel", left_wheel_parameters, left_wheel_returns, left_wheel_callback);
    printf("result: %d\n", (int)result);

    printf("registering \"right_wheel\" function");
    result = RegisterFunction(handle, "right_wheel", right_wheel_parameters, right_wheel_returns, right_wheel_callback);
    printf("result: %d\n", (int)result);

    printf("registering \"forward_backward\" axis");
    result = RegisterAxis(handle, "forward_backward", -1.0, 1.0, "movement", "x", front_back_axis);
    printf("result: %d\n", (int)result);

    printf("registering \"Lift\" axis");
    result = RegisterAxis(handle, "lift", -1.0, 1.0, "lift", "y", lift_axis);
    printf("result: %d\n", (int)result);

    // TODO: SET UP CAMERA FEED (sensor) 

    printf("connecting\n");
    result = ConnectToServer(handle, "localhost", 8089);
    printf("result: %d\n", (int)result);

    for ( int i = 0; i < 10; i++) {
        sleep(1);

        printf("updating\n");
        result = LibraryUpdate(handle);
        printf("result: %d\n", (int) result);
    }

    printf("shutting down\n");
    ShutdownLibrary(handle);
}
