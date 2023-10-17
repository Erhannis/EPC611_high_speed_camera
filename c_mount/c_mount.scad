/**
Run get_deps.sh to clone dependencies into a linked folder in your home directory.

This is actually for the EPC660 Carrier board.

Print with thick walls for tapping.
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

S_MOUNT_ID = 11.8;
C_MOUNT_ID = 24.5;
MOUNT_ID = C_MOUNT_ID;

HOLE_DX = 21;

SCREW_D = 2;
SCREW_SZ = 4-1.7;
LENS_DEPTH = 5;

H1P = [-HOLE_DX/2, 0];
H2P = [HOLE_DX/2, 0];
HPs = [H1P, H2P];
LP = [0,0];

SENSOR_SQUARE_EDGE = 13;

WT = 2;
H = SCREW_SZ+LENS_DEPTH;

module parts(holes=false) {
    union() {
        for (hp = HPs) {
            echo(hp);
            if (holes) {
                translate(hp) cylinder(d=SCREW_D,h=$FOREVER,center=true);
            } else {
                translate(hp) cylinder(d=SCREW_D+2*WT,h=H);
            }
        }
        
        echo(LP);
        tz(SCREW_SZ) if (holes) {
            translate(LP) cylinder(d=MOUNT_ID,h=$FOREVER);
        } else {
            translate(LP) cylinder(d=MOUNT_ID+2*WT,h=LENS_DEPTH);
        }
    }
}

//    translate([lp[0], lp[1], 0]) cylinder(d=S_MOUNT_ID+2*WT,);


my() translate(-LP) !difference() {
    union() {
        difference() {
            tz(SCREW_SZ/2) cube([HOLE_DX+(SCREW_D+2*WT),HOLE_DX+(SCREW_D+2*WT),SCREW_SZ], center=true);
            cube([SENSOR_SQUARE_EDGE,SENSOR_SQUARE_EDGE,$FOREVER], center=true);
        }
        parts(holes=false);
        *tz(SCREW_SZ) linear_extrude(height=LENS_DEPTH) hull() projection() {
            parts(holes=false);
        }
    }
    parts(holes=true);
}