#include <stdio.h>
#include <math.h>
#include <time.h>

#define PI 3.141592653589793
#define SOLAR_MASS (4 * PI * PI)
#define N_BODIES 5

typedef struct { double x, y, z, vx, vy, vz, mass; } Body;

Body bodies[N_BODIES] = {
    {0, 0, 0, 0, 0, 0, SOLAR_MASS}, // Sun
    {4.84143144246472090, -1.16032004402742839, -0.103622044471123109,
     0.00166007664274403694*365.24, 0.00769901118419740425*365.24, -0.0000690460016972063023*365.24,
     0.000954791938424326609 * SOLAR_MASS},
    {8.34336671824457987, 4.12479856412430479, -0.403523417114321381,
     -0.00276742510726862411*365.24, 0.00499852801234917238*365.24, 0.0000230417297573763929*365.24,
     0.000285885980666130812 * SOLAR_MASS},
    {12.8943695621391310, -15.1111514016986312, -0.223307578892655734,
     0.00296460137564761618*365.24, 0.00237847173959480950*365.24, -0.0000296589568540237556*365.24,
     0.0000436624404335156298 * SOLAR_MASS},
    {15.3796971148509165, -25.9193146099879641, 0.179258772950371181,
     0.00268067772490389322*365.24, 0.00162824170038242295*365.24, -0.0000951592254519715870*365.24,
     0.0000515138902046611451 * SOLAR_MASS}
};

double energy() {
    double e = 0;
    for (int i = 0; i < N_BODIES; i++) {
        e += 0.5 * bodies[i].mass * (bodies[i].vx*bodies[i].vx + bodies[i].vy*bodies[i].vy + bodies[i].vz*bodies[i].vz);
        for (int j = i+1; j < N_BODIES; j++) {
            double dx = bodies[i].x-bodies[j].x, dy = bodies[i].y-bodies[j].y, dz = bodies[i].z-bodies[j].z;
            e -= bodies[i].mass * bodies[j].mass / sqrt(dx*dx + dy*dy + dz*dz);
        }
    }
    return e;
}

void advance(double dt) {
    for (int i = 0; i < N_BODIES; i++)
        for (int j = i+1; j < N_BODIES; j++) {
            double dx = bodies[i].x-bodies[j].x, dy = bodies[i].y-bodies[j].y, dz = bodies[i].z-bodies[j].z;
            double d2 = dx*dx + dy*dy + dz*dz;
            double mag = dt / (d2 * sqrt(d2));
            bodies[i].vx -= dx*bodies[j].mass*mag; bodies[i].vy -= dy*bodies[j].mass*mag; bodies[i].vz -= dz*bodies[j].mass*mag;
            bodies[j].vx += dx*bodies[i].mass*mag; bodies[j].vy += dy*bodies[i].mass*mag; bodies[j].vz += dz*bodies[i].mass*mag;
        }
    for (int i = 0; i < N_BODIES; i++) {
        bodies[i].x += dt*bodies[i].vx; bodies[i].y += dt*bodies[i].vy; bodies[i].z += dt*bodies[i].vz;
    }
}

int main() {
    int n = 500000;
    double px=0,py=0,pz=0;
    for (int i=0;i<N_BODIES;i++) { px+=bodies[i].vx*bodies[i].mass; py+=bodies[i].vy*bodies[i].mass; pz+=bodies[i].vz*bodies[i].mass; }
    bodies[0].vx=-px/SOLAR_MASS; bodies[0].vy=-py/SOLAR_MASS; bodies[0].vz=-pz/SOLAR_MASS;
    printf("%.9f\n", energy());
    struct timespec s, e;
    clock_gettime(CLOCK_MONOTONIC, &s);
    for (int i = 0; i < n; i++) advance(0.01);
    clock_gettime(CLOCK_MONOTONIC, &e);
    long ms = (e.tv_sec-s.tv_sec)*1000 + (e.tv_nsec-s.tv_nsec)/1000000;
    printf("%.9f\n", energy());
    printf("n-body (%d steps): %ldms\n", n, ms);
    return 0;
}
