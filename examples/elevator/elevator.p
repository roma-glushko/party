// Elevator controller — a building with 3 floors (0, 1, 2) and one elevator.
//
// Passengers press buttons, the elevator serves requests. We verify:
//   DoorSafety      — doors never open while the elevator is moving
//   MovementLiveness — the elevator always eventually arrives after starting to move
event eCallElevator: int;
event eDoorOpened: int;
event eDoorClosed: int;
event eStartedMoving: int;
event eArrivedAtFloor: int;
event eRequestComplete: int;

// The elevator accepts floor requests and serves them one at a time.
// It picks the next floor nondeterministically from its pending request set.
machine Elevator {
  var currentFloor: int;
  var doorsAreOpen: bool;
  var requests: set[int];
  var moving: bool;

  start state Idle {
    entry {
      currentFloor = 0;
      doorsAreOpen = false;
      moving = false;
      requests = default(set[int]);
      doorsAreOpen = true;
      announce eDoorOpened, 0;
    }

    on eCallElevator do (floor: int) {
      if (floor == currentFloor) {
        // Already here — serve immediately
        announce eRequestComplete, floor;
      } else {
        requests += (floor);
        if (!moving) {
          doorsAreOpen = false;
          announce eDoorClosed, currentFloor;
          goto Dispatching;
        }
      }
    }
  }

  // Pick the next floor to visit from the pending request set
  state Dispatching {
    entry {
      var target: int;
      if (sizeof(requests) == 0) {
        doorsAreOpen = true;
        announce eDoorOpened, currentFloor;
        goto Idle;
        return;
      }
      target = choose(requests);
      requests -= (((target)));
      goto Moving, target;
    }

    on eCallElevator do (floor: int) {
      requests += (floor);
    }
  }

  // Move toward the target floor, then open doors on arrival
  state Moving {
    entry (targetFloor: int) {
      moving = true;
      announce eStartedMoving, targetFloor;
      if (targetFloor > currentFloor) {
        while (currentFloor < targetFloor) {
          currentFloor = currentFloor + 1;
        }
      } else {
        while (currentFloor > targetFloor) {
          currentFloor = currentFloor - 1;
        }
      }
      moving = false;
      announce eArrivedAtFloor, currentFloor;
      doorsAreOpen = true;
      announce eDoorOpened, currentFloor;
      announce eRequestComplete, currentFloor;
      // Also serve any pending request for this floor
      if (currentFloor in requests) {
        requests -= (((currentFloor)));
      }
      if ($) {
        doorsAreOpen = false;
        announce eDoorClosed, currentFloor;
        goto Dispatching;
      } else {
        doorsAreOpen = false;
        announce eDoorClosed, currentFloor;
        goto Dispatching;
      }
    }

    on eCallElevator do (floor: int) {
      requests += (floor);
    }
  }
}

// Each passenger requests a random floor a few times, then stops.
machine Passenger {
  var elevator: machine;
  var requestCount: int;

  start state Active {
    entry (payload: machine) {
      elevator = payload;
      requestCount = 0;
      goto RequestFloor;
    }
  }

  state RequestFloor {
    entry {
      var floor: int;
      requestCount = requestCount + 1;
      if (requestCount > 2) {
        goto Done;
        return;
      }
      floor = choose(3);
      send elevator, eCallElevator, floor;
    }

    on eRequestComplete goto RequestFloor;
    on eCallElevator goto RequestFloor;
    ignore eDoorOpened, eDoorClosed, eStartedMoving, eArrivedAtFloor;
  }

  state Done {
  }
}

// Safety: doors must never open while the elevator is in motion.
spec DoorSafety observes eStartedMoving, eArrivedAtFloor, eDoorOpened {
  start state Stopped {
    on eStartedMoving goto InMotion;
    on eDoorOpened do { }
    on eArrivedAtFloor do { }
  }

  state InMotion {
    on eArrivedAtFloor goto Stopped;
    on eDoorOpened do (floor: int) {
      assert false, "doors opened while elevator is moving!";
    }
  }
}

// Liveness: once the elevator starts moving, it must eventually arrive.
spec MovementLiveness observes eStartedMoving, eArrivedAtFloor {
  start cold state Idle {
    on eStartedMoving goto WaitingForArrival;
    ignore eArrivedAtFloor;
  }

  hot state WaitingForArrival {
    on eArrivedAtFloor goto Idle;
    ignore eStartedMoving;
  }
}

// Test harness: one elevator, two passengers
machine Main {
  start state Init {
    entry {
      var elev: machine;
      elev = new Elevator();
      new Passenger(elev);
      new Passenger(elev);
    }
  }
}

test TestElevator [main = Main]: assert DoorSafety, MovementLiveness in { Main, Elevator, Passenger };
