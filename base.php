<?php

namespace Finnbar\Quests\database;

use Exception;
use pocketmine\thread\Thread;
use pmmp\thread\ThreadSafe;
use pmmp\thread\ThreadSafeArray;
use pocketmine\plugin\PluginBase;
use pocketmine\scheduler\ClosureTask;
use Throwable;

/*

Hi future developer. This file is used by safe_sql for code generation. Don't modify it manually.

*/

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

abstract class AsyncTransaction
{
    // @phpstan-ignore-next-line
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

    public function tick(): void
    {
        if ($this->isTerminated()) throw new Exception("Thread died?");
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
                // @phpstan-ignore-next-line
                $deser = unserialize($data->data);
                if ($deser instanceof Throwable) {
                    throw $deser;
                }
                ClosureStore::$closures[$data->callbackId]($deser);
                unset(ClosureStore::$closures[$data->callbackId]);
            }
        }
    }

    public bool $stop = false;

    public function onRun(): void
    {
        $conn = new \PDO($this->databaseConnector);
        while (true) {
            $t = false;
            /** @var DataEntry[] */
            $toProcess = [];
            $this->synchronized(function () use (&$toProcess, &$t) {
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
                if ($this->stop && $this->active === 0) {
                    $t = true;
                }
            });

            if ($t) {
                \gc_enable();
                unset($conn);
                break;
            };

            /** @var DataEntry[] */
            $toAdd = [];

            foreach ($toProcess as $data) {
                /** @var AsyncTransaction */
                // @phpstan-ignore-next-line
                $at = unserialize($data->data);
                try {
                    $t = new Transaction($conn);
                    $toAdd[] = new DataEntry(false, serialize($at->run($t)), $data->callbackId);
                    if ($conn->inTransaction()) $conn->commit();
                    unset($t);
                } catch (Exception $e) {
                    throw $e;
                    // @phpstan-ignore-next-line
                    $toAdd[] = new DataEntry(false, serialize($e), $data->callbackId);
                    if ($conn->inTransaction()) $conn->rollBack();
                }
                $this->synchronized(function () {
                    $this->active--;
                });
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

    public function run(AsyncTransaction $query, \Closure $onDone = null): void
    {
        if ($onDone === null) {
            $onDone = function (mixed $data) {
                if ($data instanceof Exception) throw $data;
            };
        }
        $id = 0;
        while (isset(ClosureStore::$closures[$id])) {
            $id++;
        }
        ClosureStore::$closures[$id] = $onDone;
        $thread = $this->strongest_thread();
        $thread->synchronized(function () use (&$query, &$id, &$thread) {
            // @phpstan-ignore-next-line
            $thread->data[] = new DataEntry(true, \serialize($query), $id);
        });
    }

    public function stopThreads(): void {
        foreach ($this->threads as $thread) {
            $thread->synchronized(function () use (&$thread) {
                $thread->stop = true;
            });
            if ($thread->isRunning()) {
                $thread->join();
            }
        }
    }

    public function tick(): void
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
            $thread->start();
            $this->threads[] = $thread;
        }
    }
}

class DataEntry extends ThreadSafe
{
    public function __construct(public bool $query, public mixed $data, public ?string $callbackId = null)
    {
    }
}

class ClosureStore
{
    /** @var \Closure[] */
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
