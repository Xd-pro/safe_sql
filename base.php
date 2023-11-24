<?php

abstract class TransactionBase {

    public function __construct(public PDO $db)
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