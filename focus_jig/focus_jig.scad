/**
Run get_deps.sh to clone dependencies into a linked folder in your home directory.
*/

use <deps.link/BOSL/nema_steppers.scad>
use <deps.link/BOSL/joiners.scad>
use <deps.link/BOSL/shapes.scad>
use <deps.link/erhannisScad/misc.scad>
use <deps.link/erhannisScad/auto_lid.scad>
use <deps.link/scadFluidics/common.scad>
use <deps.link/quickfitPlate/blank_plate.scad>
use <deps.link/getriebe/Getriebe.scad>
use <deps.link/gearbox/gearbox.scad>

$FOREVER = 1000;
DUMMY = false;
$fn = DUMMY ? 10 : 60;

OD = 80;
PIN_D = 0.5;
PITCH = 1.5;
GRID_T = 0.5;
T = 3;
HOOK_S = 10;

module grid() {
    intersection() {
        cylinder(d=OD, h=$FOREVER, center=true);
        union() {
            difference() {
                cylinder(d=OD,h=T);
                cylinder(d=OD-2*T,h=$FOREVER, center=true);
            }
            crotate([0,0,180]) crotate([0,0,90]) tx(OD/2-HOOK_S*sqrt(1/2)) tz(HOOK_S*sqrt(1/2)) difference() {
                rx(90) tz(-T/2) rz(-45) cube([HOOK_S,HOOK_S,T]);
                rx(90) tz(-T/2) rz(-45) ty(T) tx(T) cube([HOOK_S-2*T,HOOK_S-2*T,T]);
            }
            linear_extrude(height=GRID_T) {
                difference() {
                    square([OD,OD], center=true);
                    N = 40;
                    for (x = [-N:N]) {
                        for (y = [-N:N]) {
                            translate([x*PITCH, y*PITCH]) square([PIN_D,PIN_D],center=true);
                        }
                    }
                }
            }
        }
    }
}

grid();