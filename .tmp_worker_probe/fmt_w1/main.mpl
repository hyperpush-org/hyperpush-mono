actor worker() do
  receive do
    msg -> println("worker")
  end
end

supervisor Sup do
  strategy : one_for_one
  
  max_restarts : 1
  
  max_seconds : 5
  
  childw1do
    start:fn->spawn(worker)end
    
    restart:permanent
    
    shutdown:5000
  end
end
