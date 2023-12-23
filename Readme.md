This is just a simple example using the three-d library for Rust to
visualize a set of 3D Spaticial Transcriptom data taset 
(https://github.com/drieslab/spatial-datasets/tree/master/data/2018_starmap_3D_cortex/raw_data)

This is not a project under active development, although I am generally interested in all sort of 3D visualization techniques since my graduated student's day. (Anyone remebers VRML?)  I just post this "exercise" code for @BickhartDerek (https://x.com/BickhartDerek/status/1737025721543913714) in response to this post (https://x.com/infoecho/status/1736912711836205482) There are quite a number of oppertunity for further optimization.

It has a CSV and HDF5 reader to read the data in `test_data`.

For compiling and running with HDF5 dataset on a Apple Silico Mac with Brew installed HDF5, you
may need to 
```
DYLD_FALLBACK_LIBRARY_PATH=/opt/homebrew/Cellar/hdf5/1.14.3/lib/
DYLD_LIBRARY_PATH=/opt/homebrew/Cellar/hdf5/1.14.3/lib/
HDF5_DIR=/opt/homebrew/Cellar/hdf5/1.14.3/
```

Jason Chin
- Dec. 23, 2023
