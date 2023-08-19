/**
Run get_deps.sh to clone dependencies into a linked folder in your home directory.

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

H1P = [141, 76];
H2P = [164, 98.5];
H3P = [172, 76];

LP = [156.355, 90.315];

SCREW_D = 2;

H = 13;
T = 5;
WT = 2;

module parts(holes=false) {
    union() {
        for (hp = [H1P, H2P, H3P]) {
            echo(hp);
            if (holes) {
                translate(hp) cylinder(d=SCREW_D,h=$FOREVER,center=true);
            } else {
                translate(hp) cylinder(d=SCREW_D+2*WT,h=H);
            }
        }
        
        echo(LP);
        if (holes) {
            translate(LP) cylinder(d=S_MOUNT_ID,h=$FOREVER,center=true);
        } else {
            translate(LP) tz(H-T) cylinder(d=S_MOUNT_ID+2*WT,h=T);
        }
    }
}

//    translate([lp[0], lp[1], 0]) cylinder(d=S_MOUNT_ID+2*WT,);


my() translate(-LP) difference() {
    union() {
        parts(holes=false);
        tz(H-T) linear_extrude(height=T) hull() projection() {
            parts(holes=false);
        }
    }
    parts(holes=true);
}