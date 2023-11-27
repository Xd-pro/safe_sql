<?php

namespace Finnbar\Duels\database;

use Exception;
use pmmp\thread\Thread;
use pmmp\thread\ThreadSafe;
use pmmp\thread\ThreadSafeArray;
use pocketmine\plugin\PluginBase;
use pocketmine\scheduler\ClosureTask;

abstract class TransactionBase
{

    public function __construct(public \PDO $db)
    {
        $db->beginTransaction();
    }

    public function commit(): bool
    {
        return $this->db->commit();
    }

    public function rollBack(): bool
    {
        return $this->db->rollBack();
    }
}

abstract class AsyncTransaction extends ThreadSafe
{

    abstract public function run(Transaction $t);
}

class DatabaseThread extends Thread
{

    /** @var ThreadSafeArray<int, DataEntry> */
    public $data;

    public int $active = 0;

    public function __construct(private string $databaseConnector)
    {
        $this->data = new ThreadSafeArray;
    }

    public function tick()
    {
        /** @var DataEntry[] */
        $toProcess = [];
        $this->synchronized(function () use (&$toProcess) {
            $toPutBack = [];
            foreach ($this->data as $data) {
                if (!$data->query) {
                    $toProcess[] = $data;
                } else {
                    $toPutBack[] = $data;
                }
            }
            $this->data = ThreadSafeArray::fromArray($toPutBack);
        });
        foreach ($toProcess as $data) {
            if ($data->callbackId !== null) {
                ClosureStore::$closures[$data->callbackId](unserialize($data->data));
                unset(ClosureStore::$closures[$data->callbackId]);
            }
        }
    }

    public function run(): void
    {
        $conn = new \PDO($this->databaseConnector);
        while (true) {
            /** @var DataEntry[] */
            $toProcess = [];
            $this->synchronized(function () use (&$toProcess) {
                $toPutBack = [];
                foreach ($this->data as $data) {
                    if ($data->query) {
                        $toProcess[] = $data;
                    } else {
                        $toPutBack[] = $data;
                    }
                }
                $this->data = ThreadSafeArray::fromArray($toPutBack);
                $this->active = count($toProcess);
            });

            /** @var DataEntry[] */
            $toAdd = [];

            foreach ($toProcess as $data) {
                /** @var AsyncTransaction */
                $at = $data->data;
                try {
                    $toAdd[] = new DataEntry(false, serialize($at->run(new Transaction($conn))), $data->callbackId);
                } catch (Exception $e) {
                    $toAdd[] = new DataEntry(false, serialize($e), $data->callbackId);
                }
                $this->synchronized(function () {
                    $this->active--;
                });
                if ($conn->inTransaction()) {
                    $conn->rollBack();
                    echo "WARN: Async transaction was not closed! Call \$transaction->commit() or \$transaction->rollBack() in the AsyncTransaction. Rolled back transaction to prevent damage\n";
                }
            }
            $this->synchronized(function () use (&$toAdd) {
                foreach ($toAdd as $ta) {
                    $this->data[] = $ta;
                }
            });
            usleep(100);
        }
    }
}

class DatabasePool
{

    /** @var DatabaseThread[] $threads */
    private array $threads = [];

    public function run(AsyncTransaction $query, \Closure $onDone = null)
    {
        if ($onDone === null) {
            $onDone = function () {
            };
        }
        $id = 0;
        while (isset(ClosureStore::$closures[$id])) {
            $id++;
        }
        ClosureStore::$closures[$id] = $onDone;
        $thread = $this->strongest_thread();
        $thread->synchronized(function () use (&$query, &$id, &$thread) {
            $thread->data[] = new DataEntry(true, $query, $id);
        });
    }

    public function tick()
    {
        foreach ($this->threads as $thread) {
            $thread->tick();
        }
    }

    private function strongest_thread(): DatabaseThread
    {
        $lowest = \PHP_INT_MAX;
        $lowestThread = null;
        foreach ($this->threads as $thread) {
            if ($thread->active < $lowest) {
                $lowest = $thread->active;
                $lowestThread = $thread;
            }
        }
        if ($lowestThread === null) throw new \Exception("No threads available to process asynchronous query");
        return $lowestThread;
    }

    public function __construct(string $connectionString, int $workers = 1)
    {
        while ($workers > 0) {
            $workers--;
            $thread = new DatabaseThread($connectionString);
            $thread->start(DatabaseThread::INHERIT_CLASSES);
            $this->threads[] = $thread;
        }
    }
}

class DataEntry extends ThreadSafe
{

    public function __construct(public bool $query, public $data, public ?string $callbackId = null)
    {
    }
}

class ClosureStore
{
    public static array $closures = [];
}

class SafeSql
{
    private function __construct()
    {
    }

    public static function bootstrapPocketmine(PluginBase $plugin, string $connectionString, int $pollTicks = 4, int $workers = 1): DatabasePool
    {
        $p = new DatabasePool($connectionString, $workers);
        $plugin->getScheduler()->scheduleRepeatingTask(new ClosureTask(function () use (&$p) {
            $p->tick();
        }), $pollTicks);
        return $p;
    }
}
