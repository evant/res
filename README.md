### Res

A tool for managing android resources.

### Usage

#### Listing

You drawables
```
res ls drawable app/src/main/res

image1.png mdpi hdpi xhdpi
image2.png      hdpi xhdpi
```

Your layouts
```
res ls layout app/src/main/res

activity_main.xml
fragment_overview.xml land
fragment_detail.xml   land sw600dp
```

#### Copying/Moving

Say you have a source directory that looks like
```
source/
    drawable-mdpi/
        image1.png
        image2.png
    drawable-hdpi/
        image1.png
        image2.png
    drawable-xhdpi/
        image1.png
        image2.png
```

and the destination `app/main/res/`

```
res cp drawable source/image1.png app/main/res
```
copies just `image1.png` to `app/main/res/drawable-*`

```
res cp drawable source/image1.png app/main/res/new_image.png
```
copies and renames `image1.png` to `new_image.png`


```
res mv drawable source/image1.png app/main/res
```
moves `image1.png`

#### Invalid Character Replacement

All file names will be made valid by striping/replacing invalid characters.

* uppercase will be converted to lowercase
* `.`, `-`, and ` ` will be converted to `_`

For example,
```
res cp "source/My Badly-named Image.png" app/main/res
```
will convert the name to `my_badly_named_image.png` 
