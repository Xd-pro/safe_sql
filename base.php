<?php

use pmmp\thread\Thread;
use pmmp\thread\ThreadSafe;
use pmmp\thread\ThreadSafeArray;
use pocketmine\plugin\Plugin;
use pocketmine\plugin\PluginBase;
use pocketmine\scheduler\ClosureTask;

abstract class TransactionBase {

    public function __construct(public \PDO $db)
    {
        $db->beginTransaction();
    }

    public function commit(): bool {
        return $this->db->commit();
    }

    public function rollBack(): bool {
        return $this->db->rollBack();
    }

}

if (extension_loaded("pmmpthread")) {
    abstract class AsyncTransaction extends ThreadSafe {

        abstract public function run(Transaction $t);
    }
    
    class DatabaseThreadInternal extends Thread {
    
        /** @var ThreadSafeArray<int, DataEntry> */
        public $data;
    
        public function __construct(private string $databaseConnector)
        {
            $this->data = new ThreadSafeArray;
        }
    
        public function run(): void {
            $conn = new \PDO($this->databaseConnector);
            while (true) {
                /** @var DataEntry[] */
                $toProcess = [];
                $this->synchronized(function() use (&$toProcess) {
                    $toPutBack = [];
                    foreach ($this->data as $data) {
                        if ($data->query) {
                            $toProcess[]=$data;
                        } else {
                            $toPutBack[]=$data;
                        }
    
                    }
                    $this->data = ThreadSafeArray::fromArray($toPutBack);
                    
                });
                /** @var DataEntry[] */
                $toAdd = [];
                
                foreach ($toProcess as $data) {
                    /** @var AsyncTransaction */
                    $at = $data->data;
                    $toAdd[]= new DataEntry(false, serialize($at->run(new Transaction($conn))), $data->callbackId);
                    
                    if ($conn->inTransaction()) {
                        $conn->rollBack();
                        throw new \Exception("Async transaction was not closed! Call \$transaction->commit() or \$transaction->rollBack() in the AsyncTransaction. Rolled back transaction to prevent damage");
                    }
                }
                $this->synchronized(function() use (&$toAdd) {
                    foreach ($toAdd as $ta) {
                        $this->data[]= $ta;
                    }
                });
                usleep(100000);
            }
        }
    
    }

    class DatabaseThread {

        private DatabaseThreadInternal $thread;

        public function run(AsyncTransaction $query, \Closure $onDone = null) {
            if ($onDone === null) {
                $onDone = function() {};
            }
            $id = 0;
            while (isset(ClosureStore::$closures[$id])) {
                $id++;
            }
            ClosureStore::$closures[$id] = $onDone;
            $this->thread->synchronized(function() use (&$query, &$id) {
                $this->thread->data[]= new DataEntry(true, $query, $id);
            });
        }

        public function __construct(string $connectionString = null, DatabaseThreadInternal $internal = null)
        {
            if ($internal === null) {
                $internal = new DatabaseThreadInternal($connectionString);
            }
            $this->thread = $internal;
        }

    }
    
    class DataEntry extends ThreadSafe {
    
        public function __construct(public bool $query, public $data, public ?string $callbackId = null)
        {
            
        }
    
    }
    
    class ClosureStore {
        public static array $closures = [];
    }
    
    function tick(DatabaseThreadInternal $t) {
        /** @var DataEntry[] */
        $toProcess = [];
        $t->synchronized(function() use (&$toProcess, &$t) {
            $toPutBack = [];
            foreach ($t->data as $data) {
                if (!$data->query) {
                    $toProcess[]=$data;
                } else {
                    $toPutBack[]=$data;
                }
    
            }
            $t->data = ThreadSafeArray::fromArray($toPutBack);
        });
        foreach ($toProcess as $data) {
            if ($data->callbackId !== null) {
                ClosureStore::$closures[$data->callbackId](unserialize($data->data));
                unset(ClosureStore::$closures[$data->callbackId]);
            }
        }
        usleep(100000);
    }

    function bootstrapPocketmine(PluginBase $plugin, string $connectionString, int $pollTicks = 2): DatabaseThread {
        $t = new DatabaseThreadInternal($connectionString);
        $t->start(DatabaseThreadInternal::INHERIT_CLASSES);
        $plugin->getScheduler()->scheduleRepeatingTask(new ClosureTask(function() use (&$t) {
            tick($t);
        }), $pollTicks);
        return new DatabaseThread(null, $t);
    }
}