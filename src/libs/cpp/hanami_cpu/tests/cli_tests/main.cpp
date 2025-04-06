#include <hanami_common/logger.h>
#include <hanami_common/threading/thread.h>
#include <hanami_cpu/cpu.h>
#include <hanami_cpu/memory.h>
#include <hanami_cpu/rapl.h>
#include <unistd.h>

#include <iostream>

using namespace Hanami;

int
main()
{
    Hanami::initConsoleLogger(true);

    Hanami::ErrorContainer error;

    std::cout << "hyperthreading active: " << isHyperthreadingEnabled(error) << std::endl;
    std::cout << "hyperthreading set to false: " << changeHyperthreadingState(false, error)
              << std::endl;
    std::cout << "hyperthreading active: " << isHyperthreadingEnabled(error) << std::endl;
    std::cout << "hyperthreading set to true: " << changeHyperthreadingState(true, error)
              << std::endl;
    std::cout << "hyperthreading active: " << isHyperthreadingEnabled(error) << std::endl;

    //==============================================================================================

    std::cout << "=============================MEMORY=============================" << std::endl;
    std::cout << "total: " << getTotalMemory() << std::endl;
    std::cout << "free: " << getFreeMemory() << std::endl;
    std::cout << "page-size: " << getPageSize() << std::endl;

    //==============================================================================================

    std::cout << "=============================CPU=============================" << std::endl;

    uint64_t numberOfThreads = 0;
    uint64_t numberOfSockets = 0;
    uint64_t socketOfThread = 0;
    uint64_t siblingId = 0;
    getNumberOfCpuThreads(numberOfThreads, error);
    getNumberOfCpuPackages(numberOfSockets, error);
    getCpuPackageId(socketOfThread, 1, error);
    getCpuSiblingId(siblingId, 1, error);
    std::cout << "threads: " << numberOfThreads << std::endl;
    std::cout << "sockets: " << numberOfSockets << std::endl;
    std::cout << "socket of thead 1: " << socketOfThread << std::endl;
    std::cout << "sibling of thread 1: " << siblingId << std::endl;

    uint64_t minSpeed = 0;
    uint64_t maxSpeed = 0;
    uint64_t curSpeed = 0;
    getCurrentMinimumSpeed(minSpeed, 1, error);
    getCurrentMaximumSpeed(maxSpeed, 1, error);
    getCurrentSpeed(curSpeed, 0, error);
    std::cout << "min of thread 1: " << minSpeed << std::endl;
    std::cout << "max of thread 1: " << maxSpeed << std::endl;

    for (int i = 0; i < 5; i++) {
        std::cout << i << " ------------------" << std::endl;
        getCurrentSpeed(curSpeed, 0, error);
        std::cout << "cur of thread 0: " << curSpeed << std::endl;

        sleep(1);
    }

    std::cout << "#######################################################################"
              << std::endl;
    for (int i = 0; i < 4; i++) {
        std::cout << "set speed Min-speed to max: " << setMinimumSpeed(i, 1000000000, error)
                  << std::endl;
    }
    for (int i = 0; i < 15; i++) {
        std::cout << i << " ------------------" << std::endl;
        getCurrentSpeed(curSpeed, 0, error);
        std::cout << "cur of thread 0: " << curSpeed << std::endl;
        sleep(1);
    }
    std::cout << "#######################################################################"
              << std::endl;

    for (int i = 0; i < 4; i++) {
        resetSpeed(i, error);
        std::cout << "set speed to min: " << setMaximumSpeed(i, 400000, error) << std::endl;
    }

    for (int i = 0; i < 15; i++) {
        std::cout << i << " ------------------" << std::endl;
        getCurrentMinimumSpeed(minSpeed, 0, error);
        getCurrentMaximumSpeed(maxSpeed, 0, error);
        getCurrentSpeed(curSpeed, 0, error);
        std::cout << "cur of thread 0: " << curSpeed << std::endl;
        std::cout << "min of thread 0: " << minSpeed << std::endl;
        std::cout << "max of thread 0: " << maxSpeed << std::endl;

        sleep(1);
    }
    std::cout << "#######################################################################"
              << std::endl;

    for (int i = 0; i < 4; i++) {
        resetSpeed(i, error);
    }

    for (int i = 0; i < 5; i++) {
        std::cout << i << " ------------------" << std::endl;
        getCurrentSpeed(curSpeed, 0, error);
        std::cout << "cur of thread 0: " << curSpeed << std::endl;

        sleep(1);
    }

    std::cout << "#######################################################################"
              << std::endl;

    std::cout << "=============================Temperature============================="
              << std::endl;

    std::vector<uint64_t> ids;

    getPkgTemperatureIds(ids, error);
    std::cout << "number of ids: " << ids.size() << std::endl;

    for (int i = 0; i < 10; i++) {
        std::cout << "temp: " << getPkgTemperature(ids.at(0), error) << std::endl;
        sleep(1);
    }

    //==============================================================================================

#if (defined(__i386__)) || (defined(__x86_64__))

    std::cout << "=============================RAPL=============================" << std::endl;

    Rapl rapl(0);
    if (rapl.initRapl(error)) {
        std::cout << "info: " << rapl.getInfo().toString() << std::endl;

        for (int i = 0; i < 10; i++) {
            std::cout << i << " ------------------" << std::endl;
            RaplDiff diff = rapl.calculateDiff();
            std::cout << diff.toString() << std::endl;
            sleep(10);
        }
    }
    else {
        LOG_ERROR(error);
    }
#endif

    //==============================================================================================

    return 0;
}
