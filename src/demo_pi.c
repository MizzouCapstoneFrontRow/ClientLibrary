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

#define RIGHT_WHEEL_FORWARD 12
#define RIGHT_WHEEL_BACKWARD 13
#define RIGHT_WHEEL_PWM 14

#define LIFT_TOP_ENDSTOP 25
#define LIFT_BOTTOM_ENDSTOP 11

#define LIFT_ENCODER_PLUS 12
#define LIFT_ENCODER_MINUS 13 

int lift_speed = 0;

const char * right_wheel_parameters[][2] = {
    {"x", "double"},
    {NULL, NULL},
};
const char * right_wheel_returns[][2] = {
    {NULL, NULL},
};


void lift_axis (const double value) {
    printf("LIFT Axis got %lf.\n", value);
    lift_speed = round(value * 100.0);
}

void update_lift() {
	if(lift_speed > 0 && digitalRead(LIFT_TOP_ENDSTOP) == HIGH)
		lift_speed = 0;

	if(lift_speed == 0) {
		digitalWrite(LIFT_BACKWARD, LOW);
		digitalWrite(LIFT_FORWARD, LOW);
	}

	if(lift_speed > 0) {
		digitalWrite(LIFT_BACKWARD, LOW);
		digitalWrite(LIFT_FORWARD, HIGH);
	}

	if(lift_speed < 0) {
		digitalWrite(LIFT_BACKWARD, HIGH);
		digitalWrite(LIFT_FORWARD, LOW);
	}

	softPwmWrite(LIFT_PWM, abs(lift_speed));
}

int main() {
    ClientHandle handle = InitializeLibrary();
    bool success;


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
    success = SetName(handle, "Example_pi");
    printf("success: %d\n", (int)success);

    printf("registering \"forward_backward\" axis");
    success = RegisterAxis(handle, "forward_backward", -1.0, 1.0, front_back_axis);
    printf("success: %d\n", (int)success);

    printf("registering \"Lift\" axis");
    success = RegisterAxis(handle, "lift", -1.0, 1.0, lift_axis);
    printf("success: %d\n", (int)success);

    // TODO: SET UP CAMERA FEED (sensor) 

    printf("connecting\n");
    success = ConnectToServer(handle, "192.168.1.3", 8089);
    printf("success: %d\n", (int)success);

    while(true) {
        printf("updating\n");
        success = LibraryUpdate(handle);
        printf("success: %d\n", (int) success);
	
    }

    printf("shutting down\n");
    ShutdownLibrary(handle);
}
