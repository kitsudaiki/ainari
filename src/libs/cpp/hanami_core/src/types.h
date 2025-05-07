#ifndef TYPES_H
#define TYPES_H

#include <string>

struct TaskLink {
    std::string hexagonId = "";
    std::string datasetName = "";
    std::string datasetFilePath = "";
    std::string columnName = "";
};

enum ReturnStatus {
    OK = 0,
    INVALID_INPUT = 1,
    ERROR = 2,
};

#endif  // TYPES_H
