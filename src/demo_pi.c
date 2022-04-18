#include <wiringPi.h>
#include <stdio.h>
#include <stdlib.h>
#include <stdbool.h>
#include "client.h"
#include <unistd.h>
#include <softPwm.h>
#include <math.h>

// PI PIN DEFINITIONS
#define LIFT_FORWARD 1
#define LIFT_BACKWARD 2
#define LIFT_PWM 3

#define LEFT_FORWARD 4
#define LEFT_BACKWARD 5
#define LEFT_PWM 6

#define RIGHT_FORWARD 12
#define RIGHT_BACKWARD 13
#define RIGHT_PWM 14

#define LIFT_TOP_ENDSTOP 25
//#define LIFT_BOTTOM_ENDSTOP 11

//#define LIFT_ENCODER_PLUS 12
//#define LIFT_ENCODER_MINUS 13 

int lift_speed = 0;
int left_speed = 0;
int right_speed = 0;
double x = 0.0;
double y = 0.0;
int forbidden_direction = 0;

void lift_axis (const double value) {
    //printf("LIFT Axis got %lf.\n", value);
    lift_speed = (int) round(value * 100.0);
    if(abs(lift_speed) < 10)
	    lift_speed = 0;
}

void update_lift() {
	// check if it's okay to allow the lift to move freely again
	if(digitalRead(LIFT_TOP_ENDSTOP) == LOW)
		forbidden_direction = 0;

	if(forbidden_direction == 0 && digitalRead(LIFT_TOP_ENDSTOP) == HIGH) {
		forbidden_direction = lift_speed; // can just set forbidden direction to speed since we're just comparing if above or below zero anyways

	// stop lift from moving in forbidden direction
	if(forbidden_direction > 0 && lift_speed > 0)
		lift_speed = 0;
	if(forbidden_direction < 0 && lift_speed < 0)
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

	softPwmWrite(LIFT_PWM, (int)abs(lift_speed));
	//printf("Lift speed: %d\n", lift_speed);
}

void update_left() {
	if(left_speed == 0) {
		digitalWrite(LEFT_BACKWARD, LOW);
		digitalWrite(LEFT_FORWARD, LOW);
	}

	if(left_speed > 0) {
		digitalWrite(LEFT_BACKWARD, LOW);
		digitalWrite(LEFT_FORWARD, HIGH);
	}

	if(left_speed < 0) {
		digitalWrite(LEFT_BACKWARD, HIGH);
		digitalWrite(LEFT_FORWARD, LOW);
	}

	softPwmWrite(LEFT_PWM, (int)abs(left_speed));
	//printf("Left speed: %d\n", left_speed);
}

void update_right() {
	if(right_speed == 0) {
		digitalWrite(RIGHT_BACKWARD, LOW);
		digitalWrite(RIGHT_FORWARD, LOW);
	}

	if(right_speed > 0) {
		digitalWrite(RIGHT_BACKWARD, LOW);
		digitalWrite(RIGHT_FORWARD, HIGH);
	}

	if(right_speed < 0) {
		digitalWrite(RIGHT_BACKWARD, HIGH);
		digitalWrite(RIGHT_FORWARD, LOW);
	}

	softPwmWrite(RIGHT_PWM, (int)abs(right_speed));
	//printf("Right speed: %d\n", right_speed);
}

void x_axis(const double value)
{
    //printf("X Axis got %lf.\n", value);
	x = value;
	if(fabs(x) < 0.1)
		x = 0.0;
}

void y_axis(const double value)
{
    //printf("Y Axis got %lf.\n", value);
	y = value;
	if(fabs(x) < 0.1)
		x = 0.0;
}

void update_wheel_speeds()
{
	// following this example: http://programming.sdarobotics.org/arcade-drive/
	double xx = x * x * x;
	double yy = y * y * y;

	//printf("xx: %lf\n", xx);
	//printf("yy: %lf\n", yy);

	left_speed = (int) round((yy + xx)*100.0);
	right_speed = (int) round((yy - xx)*100.0);
}

int main() {
    ClientHandle handle = InitializeLibrary();
    enum ErrorCode result;


    // Setup WiringPI and pinmodes
    wiringPiSetup();
    pinMode(LIFT_FORWARD, OUTPUT);
    pinMode(LIFT_BACKWARD, OUTPUT);
    softPwmCreate(LIFT_PWM, 0, 100);

    pinMode(LEFT_FORWARD, OUTPUT);
    pinMode(LEFT_BACKWARD, OUTPUT);
    softPwmCreate(LEFT_PWM, 0, 100);

    pinMode(RIGHT_FORWARD, OUTPUT);
    pinMode(RIGHT_BACKWARD, OUTPUT);
    softPwmCreate(RIGHT_PWM, 0, 100);

    pinMode(LIFT_TOP_ENDSTOP, INPUT);
//    pinMode(LIFT_BOTTOM_ENDSTOP, INPUT);
//    pinMode(LIFT_ENCODER_PLUS, INPUT);
//    pinMode(LIFT_ENCODER_MINUS, INPUT);


    //puts("setting name");
    result = SetName(handle, "demo_pi");
    //printf("result: %d\n", (int)result);

    //printf("registering \"Lift\" axis");
    result = RegisterAxis(handle, "lift", -1.0, 1.0, "lift", "z", lift_axis);
    //printf("result: %d\n", (int)result);


    //printf("registering \"Lift\" axis");
    RegisterAxis(handle, "wheel turn", -1.0, 1.0, "drive", "x", x_axis);
    //printf("result: %d\n", (int)result);

    
    //printf("registering \"Lift\" axis");
    RegisterAxis(handle, "wheel speed", -1.0, 1.0, "drive", "z", y_axis);
    //printf("result: %d\n", (int)result);

    //printf("connecting\n");
    result = ConnectToServer(handle, "192.168.1.7", 45575);
    //printf("result: %d\n", (int)result);

    if(result != NoError)
	    exit(-1);

    while(true) {
        result = LibraryUpdate(handle);
	update_lift();
	update_wheel_speeds();
	update_left();
	update_right();
	//delay(10);
    }

    //printf("shutting down\n");
    ShutdownLibrary(handle);
}
