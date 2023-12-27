# LAS+KML to STL
## What does this library do?
This library take Lidar data like freely available [data from the US Geolocical Survey](https://apps.nationalmap.gov/downloader/) and any number of KML files to create a 3d printable mesh of the terrain in the specified area.
## So it's TouchTerrain but slower?
Yes. But also I couldn't get [TouchTerrain](https://github.com/ChHarding/TouchTerrain_for_CAGEO) to work, so this library just implements it in Rust with proper (ish) error handling and **Much** better documentation in the code. It also uses local LAS files instead of going through Google Earth explorer because google didn't reply to my request :(. In addition to local file 'support', It also has support for KML files as masks. More on this in KML section.
## KML/Geo support
This library also lets you create masks for 3d data based off of KML files. You can import files containing Polygons, Trails, and Waypoints. Internally the KML files are turned into [Geo types](https://github.com/georust/geo) so you don't have to use KML files if you provide a valid [Polygon](https://docs.rs/geo/latest/geo/geometry/struct.Polygon.html), [LineString](https://docs.rs/geo/latest/geo/geometry/struct.LineString.html), or [Point](https://docs.rs/geo/latest/geo/geometry/struct.Point.html). These masks can be used to select the portion of the data to turn into an STL and adjust height values using the mask. These features together let you print paths in different colors, or even split the trails into a separate object that can be slotted into the terrain model. (letting you print the object separately and hence use different colors even without a material switching printer)
## Speed
This library is pretty slow. Straight up. Loading LAS data has not yet been implemented for multithreading and can take up to an hour if you want to load a whole country or something. However, processed height maps can be saved and loaded pretty quickly, allowing you to change STL setting with a fairly quick iteration time.
## Coordinate Systems
This library uses [UTM](https://en.wikipedia.org/wiki/Universal_Transverse_Mercator_coordinate_system) and normal Lat, Lon GPS decimal degrees. Be careful about what units various objects are, because all units use the same Structs. Please also check the units of your LAS input because while it's probably in UTM, it might be in some abomination conjured out of the ass of your local city officials. (Like California 2 SP83 survey feet). If you are unlucky enough to have data in a deprecated standard you can still load it as a height map, but then you need to convert the bounding coordinates to UTM. I recommend [espg.io](https://epsg.io/) for this, as they support many deprecated systems.
## UTM zone issues
Currently this library assumes the whole region is in the same [UTM zone](https://en.wikipedia.org/wiki/Universal_Transverse_Mercator_coordinate_system#/media/File:Utm-zones-USA.svg), but expect this to be fixed soon.
