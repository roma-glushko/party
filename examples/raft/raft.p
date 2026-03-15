enum Role { Follower, Candidate, Leader }
type LogEntry = (term: int, value: int);

event eRequestVote: (candidateId: machine, term: int, lastLogIndex: int, lastLogTerm: int);
event eVoteResponse: (term: int, granted: bool);
event eAppendEntries: (leaderId: machine, term: int, prevLogIndex: int, prevLogTerm: int, entryVal: int, leaderCommit: int);
event eAppendEntriesResponse: (term: int, success: bool, matchIdx: int);
event eTimeout;
event eClientRequest: int;
event eSetPeers: seq[machine];

machine RaftNode {
  var myId: int;
  var peers: seq[machine];
  var currentTerm: int;
  var votedFor: machine;
  var hasVotedFor: bool;
  var votedInTerm: int;
  var role: Role;
  var log: seq[LogEntry];
  var commitIndex: int;
  var votesReceived: int;
  var timer: machine;

  start state WaitForPeers {
    entry (payload: int) {
      myId = payload;
      currentTerm = 0;
      hasVotedFor = false;
      votedInTerm = -1;
      role = Follower;
      commitIndex = -1;
      votesReceived = 0;
      log = default(seq[LogEntry]);
    }

    on eSetPeers do (payload: seq[machine]) {
      peers = payload;
      timer = new RaftTimer(this);
      send timer, eTimeout;
      goto FollowerState;
    }
    ignore eTimeout, eRequestVote, eAppendEntries, eVoteResponse, eAppendEntriesResponse, eClientRequest;
  }

  state FollowerState {
    entry { role = Follower; }

    on eTimeout do {
      goto CandidateState;
    }
    on eRequestVote do (payload: (candidateId: machine, term: int, lastLogIndex: int, lastLogTerm: int)) {
      if (payload.term > currentTerm) {
        currentTerm = payload.term;
        hasVotedFor = false;
      }
      if (payload.term > currentTerm) {
        currentTerm = payload.term;
        votedFor = payload.candidateId;
        hasVotedFor = true;
        votedInTerm = payload.term;
        send payload.candidateId, eVoteResponse, (term = currentTerm, granted = true);
        send timer, eTimeout;
      } else if (payload.term == currentTerm && votedInTerm < currentTerm) {
        votedFor = payload.candidateId;
        hasVotedFor = true;
        votedInTerm = currentTerm;
        send payload.candidateId, eVoteResponse, (term = currentTerm, granted = true);
        send timer, eTimeout;
      } else {
        send payload.candidateId, eVoteResponse, (term = currentTerm, granted = false);
      }
    }
    on eAppendEntries do (payload: (leaderId: machine, term: int, prevLogIndex: int, prevLogTerm: int, entryVal: int, leaderCommit: int)) {
      if (payload.term < currentTerm) {
        send payload.leaderId, eAppendEntriesResponse, (term = currentTerm, success = false, matchIdx = -1);
        return;
      }
      currentTerm = payload.term;
      hasVotedFor = false;
      if (payload.entryVal >= 0) {
        log += (sizeof(log), (term = currentTerm, value = payload.entryVal));
      }
      if (payload.leaderCommit > commitIndex) {
        commitIndex = payload.leaderCommit;
      }
      send payload.leaderId, eAppendEntriesResponse, (term = currentTerm, success = true, matchIdx = sizeof(log) - 1);
      send timer, eTimeout;
    }
    ignore eVoteResponse, eAppendEntriesResponse, eClientRequest;
  }

  state CandidateState {
    entry {
      var i: int;
      var lastLogIdx: int;
      var lastLogTrm: int;
      role = Candidate;
      currentTerm = currentTerm + 1;
      votedFor = this;
      hasVotedFor = true;
      votedInTerm = currentTerm;
      votesReceived = 1;
      lastLogIdx = sizeof(log) - 1;
      if (sizeof(log) > 0) {
        lastLogTrm = log[sizeof(log) - 1].term;
      } else {
        lastLogTrm = 0;
      }
      i = 0;
      while (i < sizeof(peers)) {
        if (peers[i] != this) {
          send peers[i], eRequestVote, (candidateId = this, term = currentTerm, lastLogIndex = lastLogIdx, lastLogTerm = lastLogTrm);
        }
        i = i + 1;
      }
      send timer, eTimeout;
    }

    on eVoteResponse do (payload: (term: int, granted: bool)) {
      if (payload.term > currentTerm) {
        currentTerm = payload.term;
        hasVotedFor = false;
        goto FollowerState;
      }
      if (payload.granted) {
        votesReceived = votesReceived + 1;
        if (votesReceived > (sizeof(peers)) / 2) {
          goto LeaderState;
        }
      }
    }
    on eTimeout do {
      goto CandidateState;
    }
    on eRequestVote do (payload: (candidateId: machine, term: int, lastLogIndex: int, lastLogTerm: int)) {
      if (payload.term > currentTerm) {
        currentTerm = payload.term;
        hasVotedFor = true;
        votedFor = payload.candidateId;
        votedInTerm = payload.term;
        send payload.candidateId, eVoteResponse, (term = currentTerm, granted = true);
        goto FollowerState;
      }
      send payload.candidateId, eVoteResponse, (term = currentTerm, granted = false);
    }
    on eAppendEntries do (payload: (leaderId: machine, term: int, prevLogIndex: int, prevLogTerm: int, entryVal: int, leaderCommit: int)) {
      if (payload.term >= currentTerm) {
        currentTerm = payload.term;
        hasVotedFor = false;
        goto FollowerState;
      }
    }
    ignore eAppendEntriesResponse, eClientRequest;
  }

  state LeaderState {
    entry {
      var i: int;
      role = Leader;
      i = 0;
      while (i < sizeof(peers)) {
        if (peers[i] != this) {
          send peers[i], eAppendEntries, (leaderId = this, term = currentTerm, prevLogIndex = -1, prevLogTerm = 0, entryVal = -1, leaderCommit = commitIndex);
        }
        i = i + 1;
      }
      send timer, eTimeout;
    }

    on eTimeout do {
      var i: int;
      i = 0;
      while (i < sizeof(peers)) {
        if (peers[i] != this) {
          send peers[i], eAppendEntries, (leaderId = this, term = currentTerm, prevLogIndex = -1, prevLogTerm = 0, entryVal = -1, leaderCommit = commitIndex);
        }
        i = i + 1;
      }
      send timer, eTimeout;
    }
    on eClientRequest do (payload: int) {
      var i: int;
      var lastVal: int;
      log += (sizeof(log), (term = currentTerm, value = payload));
      lastVal = payload;
      i = 0;
      while (i < sizeof(peers)) {
        if (peers[i] != this) {
          send peers[i], eAppendEntries, (leaderId = this, term = currentTerm, prevLogIndex = sizeof(log) - 2, prevLogTerm = currentTerm, entryVal = lastVal, leaderCommit = commitIndex);
        }
        i = i + 1;
      }
    }
    on eAppendEntriesResponse do (payload: (term: int, success: bool, matchIdx: int)) {
      if (payload.term > currentTerm) {
        currentTerm = payload.term;
        hasVotedFor = false;
        goto FollowerState;
      }
      if (payload.success && payload.matchIdx > commitIndex) {
        commitIndex = payload.matchIdx;
      }
    }
    on eRequestVote do (payload: (candidateId: machine, term: int, lastLogIndex: int, lastLogTerm: int)) {
      if (payload.term > currentTerm) {
        currentTerm = payload.term;
        hasVotedFor = false;
        goto FollowerState;
      }
      send payload.candidateId, eVoteResponse, (term = currentTerm, granted = false);
    }
    on eAppendEntries do (payload: (leaderId: machine, term: int, prevLogIndex: int, prevLogTerm: int, entryVal: int, leaderCommit: int)) {
      if (payload.term > currentTerm) {
        currentTerm = payload.term;
        hasVotedFor = false;
        goto FollowerState;
      }
    }
    ignore eVoteResponse;
  }
}

machine RaftTimer {
  var target: machine;

  start state Active {
    entry (payload: machine) { target = payload; }

    on eTimeout do {
      if ($) {
        send target, eTimeout;
      }
    }
  }
}

spec SingleLeaderPerTerm observes eAppendEntries {
  var leaderPerTerm: map[int, machine];

  start state Monitoring {
    on eAppendEntries do (payload: (leaderId: machine, term: int, prevLogIndex: int, prevLogTerm: int, entryVal: int, leaderCommit: int)) {
      if (payload.term in leaderPerTerm) {
        assert leaderPerTerm[payload.term] == payload.leaderId, format("Safety violation: two leaders in term {0}", payload.term);
      } else {
        leaderPerTerm[payload.term] = payload.leaderId;
      }
    }
  }
}

machine Main {
  start state Setup {
    entry {
      var n1: machine;
      var n2: machine;
      var n3: machine;
      var allNodes: seq[machine];
      n1 = new RaftNode(0);
      n2 = new RaftNode(1);
      n3 = new RaftNode(2);
      allNodes = default(seq[machine]);
      allNodes += (0, n1);
      allNodes += (1, n2);
      allNodes += (2, n3);
      send n1, eSetPeers, allNodes;
      send n2, eSetPeers, allNodes;
      send n3, eSetPeers, allNodes;
      send n1, eClientRequest, 42;
      send n1, eClientRequest, 100;
    }
  }
}

test TestRaft [main = Main]: assert SingleLeaderPerTerm in { Main, RaftNode, RaftTimer };
